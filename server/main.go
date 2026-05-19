package main

import (
	"context"
	"crypto/ecdsa"
	"crypto/tls"
	"crypto/x509"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"path/filepath"
	"syscall"
	"time"

	"github.com/mehulkrdev/Assignment/server/pkg/api"
	"github.com/mehulkrdev/Assignment/server/pkg/certutil"
	"github.com/mehulkrdev/Assignment/server/pkg/enrollment"
	"github.com/mehulkrdev/Assignment/server/pkg/logger"
	"github.com/mehulkrdev/Assignment/server/pkg/pathutil"
)

var (
	caCertDER         []byte
	caPriv            *ecdsa.PrivateKey
	enrollmentService *enrollment.EnrollmentService
	serverAPI         *api.ServerAPI
)

func run() error {
	logger.InitLogger("server")
	defer logger.CleanupLogDir()

	dataDir, err := pathutil.GetDataPath()
	if err != nil {
		return fmt.Errorf("failed to get data directory: %w", err)
	}

	caCertPath := filepath.Join(dataDir, "ca.crt")
	serverCertPath := filepath.Join(dataDir, "server.crt")
	serverKeyPath := filepath.Join(dataDir, "server.key")

	// Load or Generate CA
	caCertDER, caPriv, err = certutil.LoadOrCreateCACert(caCertPath)
	if err != nil {
		return fmt.Errorf("failed to load or generate CA: %w", err)
	}
	caPEM := certutil.EncodeCertToPEM(caCertDER)

	// Load or Generate Server Cert
	serverCert, serverKey, err := certutil.LoadOrCreateServerCert(serverCertPath, serverKeyPath, caCertDER, caPriv)
	if err != nil {
		return fmt.Errorf("failed to load or generate server cert: %w", err)
	}

	serverCertPEM := certutil.EncodeCertToPEM(serverCert)
	serverKeyPEM, err := certutil.EncodePrivKeyToPEM(serverKey)
	if err != nil {
		return fmt.Errorf("failed to encode server private key to PEM: %w", err)
	}

	// Save CA cert for agent to trust
	if err := pathutil.AtomicWriteFile(caCertPath, []byte(caPEM), 0644); err != nil {
		return fmt.Errorf("failed to save CA certificate: %w", err)
	}

	if err := pathutil.AtomicWriteFile(serverCertPath, []byte(serverCertPEM), 0644); err != nil {
		return fmt.Errorf("failed to save server certificate: %w", err)
	}
	if err := pathutil.AtomicWriteFile(serverKeyPath, []byte(serverKeyPEM), 0600); err != nil {
		return fmt.Errorf("failed to save server key: %w", err)
	}

	// Initialize Enrollment Service
	enrollmentService = enrollment.NewEnrollmentService(caCertDER, caPriv, serverCert, serverKey)
	serverAPI = api.NewServerAPI(enrollmentService)

	// Setup HTTP server for enrollment (HTTPS)
	mux8443 := http.NewServeMux()
	mux8443.HandleFunc("/enroll", serverAPI.EnrollmentHandler)

	server8443 := &http.Server{
		Addr:    ":8443",
		Handler: mux8443,
	}

	// Setup mTLS server
	caCertPool := x509.NewCertPool()
	caCertPool.AppendCertsFromPEM([]byte(caPEM))

	tlsConfig := &tls.Config{
		ClientCAs:  caCertPool,
		ClientAuth: tls.RequireAndVerifyClientCert,
	}

	mux8444 := http.NewServeMux()
	mux8444.HandleFunc("/secure", func(w http.ResponseWriter, r *http.Request) {
		logger.Logf("Secure mTLS request from %s", r.RemoteAddr)
		if r.TLS != nil && len(r.TLS.PeerCertificates) > 0 {
			fmt.Fprintf(w, "Hello verified agent: %s", r.TLS.PeerCertificates[0].Subject.CommonName)
		} else {
			http.Error(w, "No client certificate", http.StatusUnauthorized)
		}
	})

	server8444 := &http.Server{
		Addr:      ":8444",
		TLSConfig: tlsConfig,
		Handler:   mux8444,
	}

	// Create a context that is cancelled when the OS sends an interrupt signal.
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	// Start servers in goroutines
	go func() {
		logger.Logf("Enrollment Server starting on port :8443 (HTTPS)")
		if err := server8443.ListenAndServeTLS(serverCertPath, serverKeyPath); err != nil && err != http.ErrServerClosed {
			logger.Logf("Enrollment Server failed to start: %v", err)
		}
	}()

	go func() {
		logger.Logf("mTLS Server starting on port :8444")
		if err := server8444.ListenAndServeTLS(serverCertPath, serverKeyPath); err != nil && err != http.ErrServerClosed {
			logger.Logf("mTLS Server failed to start: %v", err)
		}
	}()

	// Wait for interrupt signal
	<-ctx.Done()
	logger.Logf("Shutting down servers...")

	// Create a deadline to wait for servers to shutdown.
	shutdownCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	_ = server8443.Shutdown(shutdownCtx)
	_ = server8444.Shutdown(shutdownCtx)
	logger.Logf("Servers gracefully stopped.")
	return nil
}

func main() {
	if err := run(); err != nil {
		logger.Logf("Server exited with error: %v", err)
		os.Exit(1)
	}
}

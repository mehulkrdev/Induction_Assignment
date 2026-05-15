package main

import (
	"crypto/ecdsa"
	"crypto/tls"
	"crypto/x509"
	"encoding/json"
	"fmt"
	"io/ioutil"
	"net/http"
	"os"

	"github.com/mehulkrdev/Assignment/server/pkg/certutil"
	"github.com/mehulkrdev/Assignment/server/pkg/enrollment"
	"github.com/mehulkrdev/Assignment/server/pkg/logger"
)

var (
	caCertDER         []byte
	caPriv            *ecdsa.PrivateKey
	enrollmentService *enrollment.EnrollmentService
)

// EnrollmentHandler is the exported handler for the /enroll endpoint
func EnrollmentHandler(w http.ResponseWriter, r *http.Request) {
	logger.Logf("Received request for %s %s from %s", r.Method, r.URL.Path, r.RemoteAddr)

	// Only allow POST requests
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		logger.Logf("Method not allowed: %s", r.Method)
		return
	}

	var req enrollment.EnrollmentRequest
	err := json.NewDecoder(r.Body).Decode(&req)
	if err != nil {
		http.Error(w, "Malformed JSON", http.StatusBadRequest)
		logger.Logf("Malformed JSON: %v", err)
		return
	}

	clientCertPEM, err := enrollmentService.Enroll(req.AgentID, req.EnrollmentToken, req.PublicKey)
	if err != nil {
		// Map service-layer errors to appropriate HTTP status codes
		switch err.Error() {
		case "missing or invalid enrollment token":
			http.Error(w, err.Error(), http.StatusUnauthorized)
		case "missing agent_id", "invalid public key": // certutil.PublicKeyError is wrapped in enrollment service
			http.Error(w, err.Error(), http.StatusBadRequest)
		case "agent ID already enrolled":
			http.Error(w, err.Error(), http.StatusConflict)
		default:
			http.Error(w, "Failed to enroll agent", http.StatusInternalServerError)
		}
		logger.Logf("Enrollment failed for AgentID %s: %v", req.AgentID, err)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(enrollment.EnrollmentResponse{
		Status:      "success",
		Certificate: clientCertPEM,
	})
}

func main() {
	logger.InitLogger("server")
	defer logger.CleanupLogDir()

	// Initialize CA
	var err error
	caCertDER, caPriv, err = certutil.GenerateCACert()
	if err != nil {
		logger.Logf("Failed to generate CA: %v", err)
		os.Exit(1)
	}

	// Initialize Enrollment Service
	enrollmentService = enrollment.NewEnrollmentService(caCertDER, caPriv)

	// Save CA cert for agent to trust
	caPEM := certutil.EncodeCertToPEM(caCertDER)
	err = ioutil.WriteFile("ca.crt", []byte(caPEM), 0644)
	if err != nil {
		logger.Logf("Failed to save CA cert: %v", err)
		os.Exit(1)
	}

	// Generate server cert
	serverCertDER, serverPriv, err := certutil.GenerateServerCert(caCertDER, caPriv)
	if err != nil {
		logger.Logf("Failed to generate server cert: %v", err)
		os.Exit(1)
	}

	serverCertPEM := certutil.EncodeCertToPEM(serverCertDER)
	serverPrivPEM, _ := certutil.EncodePrivKeyToPEM(serverPriv)

	err = ioutil.WriteFile("server.crt", []byte(serverCertPEM), 0644)
	if err != nil {
		logger.Logf("Failed to save server cert: %v", err)
		os.Exit(1)
	}
	err = ioutil.WriteFile("server.key", []byte(serverPrivPEM), 0600)
	if err != nil {
		logger.Logf("Failed to save server key: %v", err)
		os.Exit(1)
	}

	http.HandleFunc("/enroll", EnrollmentHandler)

	port := ":8443"
	logger.Logf("Server starting on port %s (HTTPS)", port)

	// Start enrollment server in goroutine
	go func() {
		err = http.ListenAndServeTLS(port, "server.crt", "server.key", nil)
		if err != nil {
			logger.Logf("Server failed to start: %v", err)
			os.Exit(1)
		}
	}()

	// Setup mTLS server on 8444
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

	logger.Logf("mTLS Server starting on port :8444")
	err = server8444.ListenAndServeTLS("server.crt", "server.key")
	if err != nil {
		logger.Logf("mTLS Server failed to start: %v", err)
		os.Exit(1)
	}
}

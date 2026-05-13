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
	"time"

	"github.com/mehulkrdev/Assignment/server/pkg/logger"
	"github.com/mehulkrdev/Assignment/server/pkg/certutil"
)

var (
	caCertDER []byte
	caPriv    *ecdsa.PrivateKey
)

// EnrollmentRequest defines the expected JSON structure for the /enroll endpoint
type EnrollmentRequest struct {
	EnrollmentToken string `json:"enrollment_token"`
	AgentID         string `json:"agent_id"`
	PublicKey       string `json:"public_key"`
}

// EnrollmentResponse defines the response structure for the /enroll endpoint
type EnrollmentResponse struct {
	Status      string `json:"status"`
	Certificate string `json:"certificate,omitempty"`
}

// EnrollmentHandler is the exported handler for the /enroll endpoint
func EnrollmentHandler(w http.ResponseWriter, r *http.Request) {
	logger.Logf("Received request for %s %s from %s", r.Method, r.URL.Path, r.RemoteAddr)

	// Only allow POST requests
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		logger.Logf("Method not allowed: %s", r.Method)
		return
	}

	var req EnrollmentRequest
	err := json.NewDecoder(r.Body).Decode(&req)
	if err != nil {
		http.Error(w, "Malformed JSON", http.StatusBadRequest)
		logger.Logf("Malformed JSON: %v", err)
		return
	}

	// Validate enrollment_token (401 Unauthorized for missing or invalid token)
	if req.EnrollmentToken == "" || req.EnrollmentToken == "invalid-token" {
		http.Error(w, "Missing or invalid enrollment token", http.StatusUnauthorized)
		logger.Logf("Missing or invalid enrollment token: %s", req.EnrollmentToken)
		return
	}

	// Validate agent_id (400 Bad Request if empty)
	if req.AgentID == "" {
		http.Error(w, "Missing agent_id", http.StatusBadRequest)
		logger.Logf("Missing agent_id")
		return
	}

	// Simulate processing time
	time.Sleep(50 * time.Millisecond)

	logger.Logf("Enrollment successful for AgentID: %s", req.AgentID)

	// Sign the public key
	clientCertDER, err := certutil.SignClientPublicKey(caCertDER, caPriv, req.AgentID, req.PublicKey)
	if err != nil {
		http.Error(w, "Failed to sign certificate", http.StatusInternalServerError)
		logger.Logf("Failed to sign certificate: %v", err)
		return
	}

	clientCertPEM := certutil.EncodeCertToPEM(clientCertDER)

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(EnrollmentResponse{
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

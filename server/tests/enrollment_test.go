package main

import (
	"bytes"
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/x509"
	"encoding/json"
	"encoding/pem"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"testing"

	"Assignment/logger/server"
	"Assignment/server/pkg/certutil"
)

func TestMain(m *testing.M) {
	// Initialize logger for tests
	logger.InitLogger("test-server")

	// Initialize CA for tests
	var err error
	caCertDER, caPriv, err = certutil.GenerateCACert()
	if err != nil {
		fmt.Printf("Failed to generate CA: %v\n", err)
		os.Exit(1)
	}

	exitCode := m.Run()

	logger.CleanupLogDir()
	os.Exit(exitCode)
}

func TestEnrollmentEndpoint(t *testing.T) {
	t.Run("Valid Enrollment Request", func(t *testing.T) {
		// Generate a dummy ECDSA public key for the test
		priv, _ := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
		pubBytes, _ := x509.MarshalPKIXPublicKey(&priv.PublicKey)
		pubPEM := string(pem.EncodeToMemory(&pem.Block{Type: "PUBLIC KEY", Bytes: pubBytes}))

		reqBody, _ := json.Marshal(EnrollmentRequest{
			EnrollmentToken: "valid-token",
			AgentID:         "agent-1",
			PublicKey:       pubPEM,
		})
		req, _ := http.NewRequest("POST", "/enroll", bytes.NewBuffer(reqBody))
		rr := httptest.NewRecorder()

		handler := http.HandlerFunc(EnrollmentHandler)
		handler.ServeHTTP(rr, req)

		if status := rr.Code; status != http.StatusOK {
			t.Errorf("handler returned wrong status code: got %v want %v", status, http.StatusOK)
		}

		var resp EnrollmentResponse
		if err := json.NewDecoder(rr.Body).Decode(&resp); err != nil {
			t.Errorf("failed to decode response: %v", err)
		}
		if resp.Status != "success" {
			t.Errorf("expected status success, got %s", resp.Status)
		}
		if resp.Certificate == "" {
			t.Errorf("expected certificate in response, got empty")
		}
	})

	t.Run("Malformed JSON", func(t *testing.T) {
		req, _ := http.NewRequest("POST", "/enroll", bytes.NewBuffer([]byte(`{"invalid": json`)))
		rr := httptest.NewRecorder()
		handler := http.HandlerFunc(EnrollmentHandler)
		handler.ServeHTTP(rr, req)

		if status := rr.Code; status != http.StatusBadRequest {
			t.Errorf("handler returned wrong status code for malformed JSON: got %v want %v", status, http.StatusBadRequest)
		}
	})

	t.Run("Missing Enrollment Token", func(t *testing.T) {
		reqBody, _ := json.Marshal(EnrollmentRequest{
			AgentID:   "agent-1",
			PublicKey: "key-data",
		})
		req, _ := http.NewRequest("POST", "/enroll", bytes.NewBuffer(reqBody))
		rr := httptest.NewRecorder()
		handler := http.HandlerFunc(EnrollmentHandler)
		handler.ServeHTTP(rr, req)

		if status := rr.Code; status != http.StatusUnauthorized {
			t.Errorf("handler returned wrong status code for missing token: got %v want %v", status, http.StatusUnauthorized)
		}
	})

	t.Run("Empty Agent ID", func(t *testing.T) {
		reqBody, _ := json.Marshal(EnrollmentRequest{
			EnrollmentToken: "valid-token",
			AgentID:         "",
			PublicKey:       "key-data",
		})
		req, _ := http.NewRequest("POST", "/enroll", bytes.NewBuffer(reqBody))
		rr := httptest.NewRecorder()
		handler := http.HandlerFunc(EnrollmentHandler)
		handler.ServeHTTP(rr, req)

		if status := rr.Code; status != http.StatusBadRequest {
			t.Errorf("handler returned wrong status code for empty agent ID: got %v want %v", status, http.StatusBadRequest)
		}
	})
}

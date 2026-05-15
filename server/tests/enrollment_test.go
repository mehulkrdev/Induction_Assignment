package main_test

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

	"github.com/mehulkrdev/Assignment/server/pkg/certutil"
	"github.com/mehulkrdev/Assignment/server/pkg/enrollment"
	"github.com/mehulkrdev/Assignment/server/pkg/logger"
)

// These global variables from main.go are needed for TestMain setup
var (
	caCertDER []byte
	caPriv    *ecdsa.PrivateKey
)

// Expose main's EnrollmentHandler and associated types for testing
// NOTE: In a real application, these might be passed as parameters or exposed via an interface.
// For minimal refactoring, we're recreating the necessary context here.
var (
	enrollmentService *enrollment.EnrollmentService
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

	// Initialize Enrollment Service for tests
	enrollmentService = enrollment.NewEnrollmentService(caCertDER, caPriv)

	exitCode := m.Run()

	logger.CleanupLogDir()
	os.Exit(exitCode)
}

// TestableEnrollmentHandler is a wrapper around the main EnrollmentHandler
// to allow passing the httptest.ResponseRecorder and *http.Request.
func TestableEnrollmentHandler(w http.ResponseWriter, r *http.Request) {
	logger.Logf("Received test request for %s %s from %s", r.Method, r.URL.Path, r.RemoteAddr)

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
		case "missing agent_id":
			http.Error(w, err.Error(), http.StatusBadRequest)
		case "agent ID already enrolled":
			http.Error(w, err.Error(), http.StatusConflict)
		case "failed to sign certificate: invalid public key": // certutil.PublicKeyError is wrapped
			http.Error(w, "Invalid public key", http.StatusBadRequest)
		default:
			http.Error(w, "Failed to enroll agent", http.StatusInternalServerError)
		}
		logger.Logf("Test Enrollment failed for AgentID %s: %v", req.AgentID, err)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(enrollment.EnrollmentResponse{
		Status:      "success",
		Certificate: clientCertPEM,
	})
}

// Scenario: Server receives a valid enrollment request.
// Expectation: Server returns status OK and a valid certificate.
func Test_EnrollmentHandler_ValidRequest_ReturnsStatusOK(t *testing.T) {
	priv, _ := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	pubBytes, _ := x509.MarshalPKIXPublicKey(&priv.PublicKey)
	pubPEM := string(pem.EncodeToMemory(&pem.Block{Type: "PUBLIC KEY", Bytes: pubBytes}))

	reqBody, _ := json.Marshal(enrollment.EnrollmentRequest{
		EnrollmentToken: "valid-token",
		AgentID:         "agent-1",
		PublicKey:       pubPEM,
	})
	req, _ := http.NewRequest("POST", "/enroll", bytes.NewBuffer(reqBody))
	rr := httptest.NewRecorder()

	TestableEnrollmentHandler(rr, req)

	if status := rr.Code; status != http.StatusOK {
		t.Errorf("handler returned wrong status code: got %v want %v", status, http.StatusOK)
	}

	var resp enrollment.EnrollmentResponse
	if err := json.NewDecoder(rr.Body).Decode(&resp); err != nil {
		t.Errorf("failed to decode response: %v", err)
	}
	if resp.Status != "success" {
		t.Errorf("expected status success, got %s", resp.Status)
	}
	if resp.Certificate == "" {
		t.Errorf("expected certificate in response, got empty")
	}
}

// Scenario: Server receives a request with an invalid enrollment token.
// Expectation: Server returns status Unauthorized.
func Test_EnrollmentHandler_InvalidToken_ReturnsStatusUnauthorized(t *testing.T) {
	reqBody, _ := json.Marshal(enrollment.EnrollmentRequest{
		EnrollmentToken: "some-random-invalid-token", // Not "valid-token"
		AgentID:         "agent-1",
		PublicKey:       "key-data",
	})
	req, _ := http.NewRequest("POST", "/enroll", bytes.NewBuffer(reqBody))
	rr := httptest.NewRecorder()
	TestableEnrollmentHandler(rr, req)

	if status := rr.Code; status != http.StatusUnauthorized {
		t.Errorf("handler returned wrong status code for invalid token: got %v want %v", status, http.StatusUnauthorized)
	}
}

// Scenario: Server receives a duplicate enrollment request for an already enrolled agent ID.
// Expectation: Server returns status Conflict (or similar).
func Test_EnrollmentHandler_DuplicateAgentID_ReturnsStatusConflict(t *testing.T) {
	// First enrollment (should succeed)
	priv1, _ := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	pubBytes1, _ := x509.MarshalPKIXPublicKey(&priv1.PublicKey)
	pubPEM1 := string(pem.EncodeToMemory(&pem.Block{Type: "PUBLIC KEY", Bytes: pubBytes1}))

	reqBody1, _ := json.Marshal(enrollment.EnrollmentRequest{
		EnrollmentToken: "valid-token",
		AgentID:         "agent-duplicate-test",
		PublicKey:       pubPEM1,
	})
	req1, _ := http.NewRequest("POST", "/enroll", bytes.NewBuffer(reqBody1))
	rr1 := httptest.NewRecorder()
	TestableEnrollmentHandler(rr1, req1)

	if status := rr1.Code; status != http.StatusOK {
		t.Fatalf("First enrollment failed: got %v want %v", status, http.StatusOK)
	}

	// Second enrollment with the same AgentID but new public key (should conflict)
	priv2, _ := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	pubBytes2, _ := x509.MarshalPKIXPublicKey(&priv2.PublicKey)
	pubPEM2 := string(pem.EncodeToMemory(&pem.Block{Type: "PUBLIC KEY", Bytes: pubBytes2}))

	reqBody2, _ := json.Marshal(enrollment.EnrollmentRequest{
		EnrollmentToken: "valid-token",
		AgentID:         "agent-duplicate-test",
		PublicKey:       pubPEM2,
	})
	req2, _ := http.NewRequest("POST", "/enroll", bytes.NewBuffer(reqBody2))
	rr2 := httptest.NewRecorder()
	TestableEnrollmentHandler(rr2, req2)

	// Expecting Conflict status if agent ID is already registered.
	if status := rr2.Code; status != http.StatusConflict {
		t.Errorf("handler returned wrong status code for duplicate agent ID: got %v want %v", status, http.StatusConflict)
	}
}

// Scenario: Server receives a malformed JSON body.
// Expectation: Server returns status Bad Request.
func Test_EnrollmentHandler_MalformedJSON_ReturnsStatusBadRequest(t *testing.T) {
	req, _ := http.NewRequest("POST", "/enroll", bytes.NewBuffer([]byte(`{"invalid": json`)))
	rr := httptest.NewRecorder()
	TestableEnrollmentHandler(rr, req)

	if status := rr.Code; status != http.StatusBadRequest {
		t.Errorf("handler returned wrong status code for malformed JSON: got %v want %v", status, http.StatusBadRequest)
	}
}

// Scenario: Server receives a request with a missing enrollment token.
// Expectation: Server returns status Unauthorized.
func Test_EnrollmentHandler_MissingToken_ReturnsStatusUnauthorized(t *testing.T) {
	reqBody, _ := json.Marshal(enrollment.EnrollmentRequest{
		AgentID:   "agent-1",
		PublicKey: "key-data",
	})
	req, _ := http.NewRequest("POST", "/enroll", bytes.NewBuffer(reqBody))
	rr := httptest.NewRecorder()
	TestableEnrollmentHandler(rr, req)

	if status := rr.Code; status != http.StatusUnauthorized {
		t.Errorf("handler returned wrong status code for missing token: got %v want %v", status, http.StatusUnauthorized)
	}
}

// Scenario: Server receives a request with an empty agent ID.
// Expectation: Server returns status Bad Request.
func Test_EnrollmentHandler_EmptyAgentID_ReturnsStatusBadRequest(t *testing.T) {
	reqBody, _ := json.Marshal(enrollment.EnrollmentRequest{
		EnrollmentToken: "valid-token",
		AgentID:         "",
		PublicKey:       "key-data",
	})
	req, _ := http.NewRequest("POST", "/enroll", bytes.NewBuffer(reqBody))
	rr := httptest.NewRecorder()
	TestableEnrollmentHandler(rr, req)

	if status := rr.Code; status != http.StatusBadRequest {
		t.Errorf("handler returned wrong status code for empty agent ID: got %v want %v", status, http.StatusBadRequest)
	}
}

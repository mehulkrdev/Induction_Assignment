package tests

import (
	"bytes"
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"encoding/json"
	"encoding/pem"
	"fmt"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync"
	"testing"

	"github.com/mehulkrdev/Assignment/server/pkg/api"
	"github.com/mehulkrdev/Assignment/server/pkg/certutil"
	"github.com/mehulkrdev/Assignment/server/pkg/enrollment"
	"github.com/mehulkrdev/Assignment/server/pkg/logger"
)

func init() {
	logger.InitLogger("server_test")
}

// generateKeyPair generates a new ECDSA private and public key pair.
func generateKeyPair() (*ecdsa.PrivateKey, string, error) {
	priv, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		return nil, "", fmt.Errorf("failed to generate private key: %w", err)
	}

	pubASN1, err := certutil.MarshalPublicKey(priv.Public())
	if err != nil {
		return nil, "", fmt.Errorf("failed to marshal public key: %w", err)
	}

	pubKeyPEM := pem.EncodeToMemory(&pem.Block{Type: "PUBLIC KEY", Bytes: pubASN1})
	return priv, string(pubKeyPEM), nil
}

func TestEnrollmentService_Enroll_Success(t *testing.T) {
	caCertDER, caPriv, err := certutil.GenerateCACert()
	if err != nil {
		t.Fatalf("Failed to generate CA: %v", err)
	}

	es := enrollment.NewEnrollmentService(caCertDER, caPriv, nil, nil)

	_, pubKeyPEM, err := generateKeyPair()
	if err != nil {
		t.Fatalf("Failed to generate key pair: %v", err)
	}

	clientCertPEM, err := es.Enroll("agent123", "valid-token", pubKeyPEM)
	if err != nil {
		t.Fatalf("Enrollment failed: %v", err)
	}

	if clientCertPEM == "" {
		t.Error("Expected a client certificate, got empty string")
	}
}

func TestEnrollmentService_Enroll_DuplicateAgentID(t *testing.T) {
	caCertDER, caPriv, err := certutil.GenerateCACert()
	if err != nil {
		t.Fatalf("Failed to generate CA: %v", err)
	}

	es := enrollment.NewEnrollmentService(caCertDER, caPriv, nil, nil)

	_, pubKeyPEM, err := generateKeyPair()
	if err != nil {
		t.Fatalf("Failed to generate key pair: %v", err)
	}

	_, err = es.Enroll("agent123", "valid-token", pubKeyPEM)
	if err != nil {
		t.Fatalf("First enrollment failed unexpectedly: %v", err)
	}

	// Second enrollment with the same agent ID should fail
	_, err = es.Enroll("agent123", "valid-token", pubKeyPEM)
	if err == nil || err.Error() != "agent ID already enrolled" {
		t.Errorf("Expected 'agent ID already enrolled' error, got: %v", err)
	}
}

func TestEnrollmentService_Enroll_Concurrent(t *testing.T) {
	caCertDER, caPriv, err := certutil.GenerateCACert()
	if err != nil {
		t.Fatalf("Failed to generate CA: %v", err)
	}

	es := enrollment.NewEnrollmentService(caCertDER, caPriv, nil, nil)

	var wg sync.WaitGroup
	numAgents := 100
	errors := make(chan error, numAgents)

	for i := 0; i < numAgents; i++ {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			agentID := fmt.Sprintf("agent%d", i)
			_, pubKeyPEM, err := generateKeyPair()
			if err != nil {
				errors <- fmt.Errorf("failed to generate key pair for %s: %w", agentID, err)
				return
			}
			_, err = es.Enroll(agentID, "valid-token", pubKeyPEM)
			if err != nil && !strings.Contains(err.Error(), "agent ID already enrolled") {
				errors <- fmt.Errorf("enrollment failed for %s: %w", agentID, err)
			}
		}(i)
	}

	wg.Wait()
	close(errors)

	for err := range errors {
		if err != nil {
			t.Errorf("Concurrent enrollment error: %v", err)
		}
	}

	if len(es.GetEnrolledAgents()) != numAgents {
		t.Errorf("Expected %d enrolled agents, got %d", numAgents, len(es.GetEnrolledAgents()))
	}
}

func TestEnrollmentHandler_Success(t *testing.T) {
	// Setup CA and EnrollmentService for the handler
	caCertDER, caPriv, err := certutil.GenerateCACert()
	if err != nil {
		t.Fatalf("Failed to generate CA: %v", err)
	}
	es := enrollment.NewEnrollmentService(caCertDER, caPriv, nil, nil)
	serverAPI := api.NewServerAPI(es)

	// Generate agent key pair
	_, pubKeyPEM, err := generateKeyPair()
	if err != nil {
		t.Fatalf("Failed to generate key pair: %v", err)
	}

	// Create request body
	reqBody := enrollment.EnrollmentRequest{
		AgentID:         "test-agent-1",
		EnrollmentToken: "valid-token",
		PublicKey:       pubKeyPEM,
	}
	jsonBody, _ := json.Marshal(reqBody)

	req := httptest.NewRequest("POST", "/enroll", bytes.NewBuffer(jsonBody))
	req.Header.Set("Content-Type", "application/json")

	rr := httptest.NewRecorder()
	serverAPI.EnrollmentHandler(rr, req)

	if status := rr.Code; status != http.StatusOK {
		t.Errorf("Handler returned wrong status code: got %v want %v, response: %s",
			status, http.StatusOK, rr.Body.String())
	}

	var res enrollment.EnrollmentResponse
	err = json.NewDecoder(rr.Body).Decode(&res)
	if err != nil {
		t.Fatalf("Failed to decode response: %v", err)
	}

	if res.Status != "success" {
		t.Errorf("Expected status 'success', got %s", res.Status)
	}
	if res.Certificate == "" {
		t.Error("Expected certificate in response, got empty")
	}
}

func TestEnrollmentHandler_InvalidMethod(t *testing.T) {
	api := api.NewServerAPI(nil)
	req := httptest.NewRequest("GET", "/enroll", nil)
	rr := httptest.NewRecorder()
	api.EnrollmentHandler(rr, req)

	if status := rr.Code; status != http.StatusMethodNotAllowed {
		t.Errorf("Handler returned wrong status code: got %v want %v",
			status, http.StatusMethodNotAllowed)
	}
}

func TestEnrollmentHandler_MalformedJSON(t *testing.T) {
	api := api.NewServerAPI(nil)
	req := httptest.NewRequest("POST", "/enroll", strings.NewReader("invalid json"))
	req.Header.Set("Content-Type", "application/json")
	rr := httptest.NewRecorder()
	api.EnrollmentHandler(rr, req)

	if status := rr.Code; status != http.StatusBadRequest {
		t.Errorf("Handler returned wrong status code: got %v want %v",
			status, http.StatusBadRequest)
	}
}

func TestEnrollmentHandler_RequestTooLarge(t *testing.T) {
	api := api.NewServerAPI(nil)

	// Create a body larger than 64KB
	largeBody := make([]byte, 64*1024+1) // 64 KB + 1 byte
	req := httptest.NewRequest("POST", "/enroll", bytes.NewBuffer(largeBody))
	req.Header.Set("Content-Type", "application/json")
	rr := httptest.NewRecorder()
	api.EnrollmentHandler(rr, req)

	if status := rr.Code; status != http.StatusBadRequest {
		t.Errorf("Handler returned wrong status code for large request: got %v want %v",
			status, http.StatusBadRequest)
	}

	if !strings.Contains(rr.Body.String(), "request too large") {
		t.Errorf("Expected error message to contain \"request too large\", got: %s", rr.Body.String())
	}
}

func TestEnrollmentHandler_InvalidToken(t *testing.T) {
	// Setup CA and EnrollmentService for the handler
	caCertDER, caPriv, err := certutil.GenerateCACert()
	if err != nil {
		t.Fatalf("Failed to generate CA: %v", err)
	}
	es := enrollment.NewEnrollmentService(caCertDER, caPriv, nil, nil)
	serverAPI := api.NewServerAPI(es)

	// Generate agent key pair
	_, pubKeyPEM, err := generateKeyPair()
	if err != nil {
		t.Fatalf("Failed to generate key pair: %v", err)
	}

	reqBody := enrollment.EnrollmentRequest{
		AgentID:         "test-agent-2",
		EnrollmentToken: "invalid-token", // Invalid token
		PublicKey:       pubKeyPEM,
	}
	jsonBody, _ := json.Marshal(reqBody)

	req := httptest.NewRequest("POST", "/enroll", bytes.NewBuffer(jsonBody))
	req.Header.Set("Content-Type", "application/json")
	rr := httptest.NewRecorder()
	serverAPI.EnrollmentHandler(rr, req)

	if status := rr.Code; status != http.StatusUnauthorized {
		t.Errorf("Handler returned wrong status code: got %v want %v",
			status, http.StatusUnauthorized)
	}
}

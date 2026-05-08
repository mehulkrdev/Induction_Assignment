package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

// EnrollmentRequest defines the expected JSON structure for the /enroll endpoint
type EnrollmentRequest struct {
	EnrollmentToken string `json:"enrollment_token"`
	AgentID         string `json:"agent_id"`
	PublicKey       string `json:"public_key"`
}

func TestEnrollmentEndpoint(t *testing.T) {
	// Re-using the logic from main.go\"s handler but for testing
	// In TDD, we want to test the handler. Since main.go has an anonymous handler,
	// we\"ll likely need to refactor main.go later. For now, we test against the intended behavior.

	t.Run("Valid Enrollment Request", func(t *testing.T) {
		reqBody, _ := json.Marshal(EnrollmentRequest{
			EnrollmentToken: "valid-token",
			AgentID:         "agent-1",
			PublicKey:       "key-data",
		})
		req, _ := http.NewRequest("POST", "/enroll", bytes.NewBuffer(reqBody))
		rr := httptest.NewRecorder()

		// This will call the actual handler once we refactor. 
		// For RED phase, we are testing against what we WANT.
		// Since main.go doesn\"t export the handler, I\"ll mock the call to the endpoint
		// as if it were processed by the server logic we\"re about to write.
		
		handler := http.HandlerFunc(EnrollmentHandler) // We expect to implement this
		handler.ServeHTTP(rr, req)

		if status := rr.Code; status != http.StatusOK {
			t.Errorf("handler returned wrong status code: got %v want %v", status, http.StatusOK)
		}
	})

	t.Run("Malformed JSON", func(t *testing.T) {
		req, _ := http.NewRequest("POST", "/enroll", bytes.NewBuffer([]byte(`{\"invalid\": json`)))
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

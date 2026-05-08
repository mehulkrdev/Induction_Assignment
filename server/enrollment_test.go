package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"testing"

	"Assignment/logger/server"
)

func TestMain(m *testing.M) {
	// Initialize logger for tests
	logger.InitLogger("test-server")

	exitCode := m.Run()

	logger.CleanupLogDir()
	os.Exit(exitCode)
}

func TestEnrollmentEndpoint(t *testing.T) {
	t.Run("Valid Enrollment Request", func(t *testing.T) {
		reqBody, _ := json.Marshal(EnrollmentRequest{
			EnrollmentToken: "valid-token",
			AgentID:         "agent-1",
			PublicKey:       "key-data",
		})
		req, _ := http.NewRequest("POST", "/enroll", bytes.NewBuffer(reqBody))
		rr := httptest.NewRecorder()

		handler := http.HandlerFunc(EnrollmentHandler)
		handler.ServeHTTP(rr, req)

		if status := rr.Code; status != http.StatusOK {
			t.Errorf("handler returned wrong status code: got %v want %v", status, http.StatusOK)
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

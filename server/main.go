package main

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"time"

	"Assignment/logger/server"
)

// EnrollmentRequest defines the expected JSON structure for the /enroll endpoint
type EnrollmentRequest struct {
	EnrollmentToken string `json:"enrollment_token"`
	AgentID         string `json:"agent_id"`
	PublicKey       string `json:"public_key"`
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

	w.WriteHeader(http.StatusOK)
	fmt.Fprintf(w, "Enrollment successful")
}

func main() {
	logger.InitLogger("server")
	defer logger.CleanupLogDir()

	http.HandleFunc("/enroll", EnrollmentHandler)

	port := ":8443"
	logger.Logf("Server starting on port %s", port)
	err := http.ListenAndServe(port, nil)
	if err != nil {
		logger.Logf("Server failed to start: %v", err)
		os.Exit(1)
	}
}

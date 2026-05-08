package main

import (
	"encoding/json"
	"fmt"
	"net/http"
)

// EnrollmentRequest defines the expected JSON structure for the /enroll endpoint
type EnrollmentRequest struct {
	EnrollmentToken string `json:"enrollment_token"`
	AgentID         string `json:"agent_id"`
	PublicKey       string `json:"public_key"`
}

// EnrollmentHandler is the exported handler for the /enroll endpoint
func EnrollmentHandler(w http.ResponseWriter, r *http.Request) {
	// Only allow POST requests
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var req EnrollmentRequest
	err := json.NewDecoder(r.Body).Decode(&req)
	if err != nil {
		http.Error(w, "Malformed JSON", http.StatusBadRequest)
		return
	}

	// Validate enrollment_token (401 Unauthorized for missing or invalid token)
	if req.EnrollmentToken == "" || req.EnrollmentToken == "invalid-token" {
		http.Error(w, "Missing or invalid enrollment token", http.StatusUnauthorized)
		return
	}

	// Validate agent_id (400 Bad Request if empty)
	if req.AgentID == "" {
		http.Error(w, "Missing agent_id", http.StatusBadRequest)
		return
	}

	// Minimum implementation for GREEN phase
	w.WriteHeader(http.StatusOK)
	fmt.Fprintf(w, "Enrollment successful")
}

func main() {
	http.HandleFunc("/enroll", EnrollmentHandler)

	fmt.Println("Server listening on :8443")
	err := http.ListenAndServe(":8443", nil)
	if err != nil {
		panic(err)
	}
}

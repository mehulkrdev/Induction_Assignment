package api

import (
	"encoding/json"
	"net/http"

	"github.com/mehulkrdev/Assignment/server/pkg/enrollment"
	"github.com/mehulkrdev/Assignment/server/pkg/logger"
)

type ServerAPI struct {
	EnrollmentService *enrollment.EnrollmentService
}

func NewServerAPI(es *enrollment.EnrollmentService) *ServerAPI {
	return &ServerAPI{
		EnrollmentService: es,
	}
}

// EnrollmentHandler is the handler for the /enroll endpoint
func (api *ServerAPI) EnrollmentHandler(w http.ResponseWriter, r *http.Request) {
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

	clientCertPEM, err := api.EnrollmentService.Enroll(req.AgentID, req.EnrollmentToken, req.PublicKey)
	if err != nil {
		// Map service-layer errors to appropriate HTTP status codes
		switch err.Error() {
		case "missing or invalid enrollment token":
			http.Error(w, err.Error(), http.StatusUnauthorized)
		case "missing agent_id", "invalid public key":
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

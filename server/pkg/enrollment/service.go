package enrollment

import (
	"crypto/ecdsa"
	"fmt"
	"os"
	"sync"

	"github.com/mehulkrdev/Assignment/server/pkg/certutil"
	"github.com/mehulkrdev/Assignment/server/pkg/logger"
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

// EnrollmentService encapsulates the business logic and state for agent enrollment.
type EnrollmentService struct {
	caCertDER      []byte
	caPriv         *ecdsa.PrivateKey
	enrolledAgents map[string]bool
	mu             sync.RWMutex
}

// NewEnrollmentService creates and initializes a new EnrollmentService.
func NewEnrollmentService(caCertDER []byte, caPriv *ecdsa.PrivateKey, serverCertDER []byte, serverPriv *ecdsa.PrivateKey) *EnrollmentService {
	return &EnrollmentService{
		caCertDER:      caCertDER,
		caPriv:         caPriv,
		enrolledAgents: make(map[string]bool),
		mu:             sync.RWMutex{},
	}
}

// Enroll performs the business logic for agent enrollment.
// It validates the token and agent ID, signs the public key, and marks the agent as enrolled.
// It returns the PEM-encoded client certificate or an error.
func (es *EnrollmentService) Enroll(agentID, token, publicKey string) (string, error) {
	// Validate enrollment_token (401 Unauthorized for missing or invalid token)
	validToken := os.Getenv("ENROLLMENT_TOKEN")
	if validToken == "" {
		return "", fmt.Errorf("server misconfiguration: ENROLLMENT_TOKEN not set")
	}
	if token == "" || token != validToken {
		logger.Logf("Missing or invalid enrollment token: %s", token)
		return "", fmt.Errorf("missing or invalid enrollment token")
	}

	// Validate agent_id (400 Bad Request if empty)
	if agentID == "" {
		logger.Logf("Missing agent_id")
		return "", fmt.Errorf("missing agent_id")
	}

	// Hold a write lock for the entire check-sign-mark sequence to prevent TOCTOU race.
	es.mu.Lock()
	defer es.mu.Unlock()

	// Check for duplicate AgentID
	if _, found := es.enrolledAgents[agentID]; found {
		logger.Logf("Duplicate enrollment attempt for AgentID: %s", agentID)
		return "", fmt.Errorf("agent ID already enrolled")
	}

	// Reserve the slot to block concurrent requests for same ID (in-progress sentinel).
	// This will be updated to true on success, or deleted on failure.
	es.enrolledAgents[agentID] = false

	// Sign the public key
	clientCertDER, err := certutil.SignClientPublicKey(es.caCertDER, es.caPriv, agentID, publicKey)
	if err != nil {
		delete(es.enrolledAgents, agentID) // Release reservation on error
		return "", fmt.Errorf("failed to sign certificate: %w", err)
	}

	// Mark agent as enrolled after successful certificate issuance.
	es.enrolledAgents[agentID] = true

	logger.Logf("Enrollment successful for AgentID: %s", agentID)

	clientCertPEM := certutil.EncodeCertToPEM(clientCertDER)

	return clientCertPEM, nil
}

// GetEnrolledAgents for testing purposes
func (es *EnrollmentService) GetEnrolledAgents() map[string]bool {
	es.mu.RLock()
	defer es.mu.RUnlock()
	// Return a copy to prevent external modification
	copyMap := make(map[string]bool)
	for k, v := range es.enrolledAgents {
		copyMap[k] = v
	}
	return copyMap
}

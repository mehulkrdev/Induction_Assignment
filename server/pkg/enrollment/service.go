package enrollment

import (
	"crypto/ecdsa"
	"fmt"

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
}

// NewEnrollmentService creates and initializes a new EnrollmentService.
func NewEnrollmentService(caCertDER []byte, caPriv *ecdsa.PrivateKey) *EnrollmentService {
	return &EnrollmentService{
		caCertDER:      caCertDER,
		caPriv:         caPriv,
		enrolledAgents: make(map[string]bool),
	}
}

// Enroll performs the business logic for agent enrollment.
// It validates the token and agent ID, signs the public key, and marks the agent as enrolled.
// It returns the PEM-encoded client certificate or an error.
func (es *EnrollmentService) Enroll(agentID, token, publicKey string) (string, error) {
	// Validate enrollment_token (401 Unauthorized for missing or invalid token)
	// For simplicity, we hardcode a valid token. In a real system, this would involve a secure token validation service.
	if token == "" || token != "valid-token" {
		logger.Logf("Missing or invalid enrollment token: %s", token)
		return "", fmt.Errorf("missing or invalid enrollment token")
	}

	// Validate agent_id (400 Bad Request if empty)
	if agentID == "" {
		logger.Logf("Missing agent_id")
		return "", fmt.Errorf("missing agent_id")
	}

	// Check for duplicate AgentID
	if _, found := es.enrolledAgents[agentID]; found {
		logger.Logf("Duplicate enrollment attempt for AgentID: %s", agentID)
		return "", fmt.Errorf("agent ID already enrolled")
	}

	logger.Logf("Enrollment successful for AgentID: %s", agentID)

	// Sign the public key
	clientCertDER, err := certutil.SignClientPublicKey(es.caCertDER, es.caPriv, agentID, publicKey)
	if err != nil {
		return "", fmt.Errorf("failed to sign certificate: %w", err)
	}

	// Mark agent as enrolled after successful certificate issuance
	es.enrolledAgents[agentID] = true

	clientCertPEM := certutil.EncodeCertToPEM(clientCertDER)

	return clientCertPEM, nil
}

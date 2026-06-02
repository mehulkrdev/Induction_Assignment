package certutil

import (
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/tls"
	"crypto/x509"
	"crypto/x509/pkix"
	"encoding/pem"
	"fmt"
	"math/big"
	"net"
	"os"
	"path/filepath"
	"time"

	"github.com/mehulkrdev/Assignment/server/pkg/logger"
	"github.com/mehulkrdev/Assignment/server/pkg/pathutil"
)

// PublicKeyError is a custom error type for public key related issues
type PublicKeyError struct {
	Message string
}

func (e *PublicKeyError) Error() string {
	return e.Message
}

// generateRandomSerialNumber generates a cryptographically secure random serial number.
func generateRandomSerialNumber() (*big.Int, error) {
	serialNumberLimit := new(big.Int).Lsh(big.NewInt(1), 128) // 2^128
	serialNumber, err := rand.Int(rand.Reader, serialNumberLimit)
	if err != nil {
		return nil, fmt.Errorf("failed to generate random serial number: %w", err)
	}
	return serialNumber, nil
}

// GenerateCACert generates a self-signed CA certificate and private key.
func GenerateCACert() ([]byte, *ecdsa.PrivateKey, error) {
	priv, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to generate CA private key: %w", err)
	}

	serialNumber, err := generateRandomSerialNumber()
	if err != nil {
		return nil, nil, fmt.Errorf("failed to generate CA certificate serial number: %w", err)
	}

	template := x509.Certificate{
		SerialNumber: serialNumber,
		Subject: pkix.Name{
			Organization: []string{"Enrollment CA"},
		},
		NotBefore:             time.Now(),
		NotAfter:              time.Now().AddDate(10, 0, 0),
		IsCA:                  true,
		ExtKeyUsage:           []x509.ExtKeyUsage{x509.ExtKeyUsageClientAuth, x509.ExtKeyUsageServerAuth},
		KeyUsage:              x509.KeyUsageDigitalSignature | x509.KeyUsageCertSign,
		BasicConstraintsValid: true,
	}

	derBytes, err := x509.CreateCertificate(rand.Reader, &template, &template, &priv.PublicKey, priv)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to create CA certificate: %w", err)
	}

	return derBytes, priv, nil
}

// LoadOrCreateCACert loads an existing CA certificate and key, or generates new ones.
func LoadOrCreateCACert(caCertPath string) ([]byte, *ecdsa.PrivateKey, error) {
	caKeyPath := filepath.Join(filepath.Dir(caCertPath), "ca.key")

	// Try to load existing CA cert and key
	caCertPEM, err := os.ReadFile(caCertPath)
	if err == nil {
		caKeyPEM, err := os.ReadFile(caKeyPath)
		if err == nil {
			// Both exist, parse them
			caCertDERBlock, _ := pem.Decode(caCertPEM)
			if caCertDERBlock == nil || caCertDERBlock.Type != "CERTIFICATE" {
				return nil, nil, fmt.Errorf("failed to decode PEM block from %s", caCertPath)
			}
			caCert, err := x509.ParseCertificate(caCertDERBlock.Bytes)
			if err != nil {
				return nil, nil, fmt.Errorf("failed to parse CA certificate from %s: %w", caCertPath, err)
			}

			caKeyDERBlock, _ := pem.Decode(caKeyPEM)
			if caKeyDERBlock == nil || caKeyDERBlock.Type != "EC PRIVATE KEY" {
				return nil, nil, fmt.Errorf("failed to decode PEM block from %s", caKeyPath)
			}
			caPriv, err := x509.ParseECPrivateKey(caKeyDERBlock.Bytes)
			if err != nil {
				return nil, nil, fmt.Errorf("failed to parse CA private key from %s: %w", caKeyPath, err)
			}

			logger.Logf("Loaded existing CA certificate from %s and %s. Subject: %s, Issuer: %s, Serial: %s, NotBefore: %s, NotAfter: %s",
				caCertPath, caKeyPath, caCert.Subject, caCert.Issuer, caCert.SerialNumber, caCert.NotBefore.Format(time.RFC3339), caCert.NotAfter.Format(time.RFC3339))
			return caCert.Raw, caPriv, nil
		}
	}

	// If not found or error, generate new CA
	logger.Logf("Generating new CA certificate and key.")
	caCertDER, caPriv, err := GenerateCACert()
	if err != nil {
		return nil, nil, fmt.Errorf("failed to generate new CA: %w", err)
	}

	// Atomically save new CA cert and key
	caPEM := EncodeCertToPEM(caCertDER)
	caPrivPEM, err := EncodePrivKeyToPEM(caPriv)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to encode CA private key to PEM: %w", err)
	}

	if err := pathutil.AtomicWriteFile(caCertPath, []byte(caPEM), 0644); err != nil {
		return nil, nil, fmt.Errorf("failed to save CA certificate: %w", err)
	}
	parsedCACert, _ := x509.ParseCertificate(caCertDER)
	logger.Logf("Saved new CA certificate to %s. Subject: %s, Issuer: %s, Serial: %s, NotBefore: %s, NotAfter: %s",
		caCertPath, parsedCACert.Subject, parsedCACert.Issuer, parsedCACert.SerialNumber, parsedCACert.NotBefore.Format(time.RFC3339), parsedCACert.NotAfter.Format(time.RFC3339))

	if err := pathutil.AtomicWriteFile(caKeyPath, []byte(caPrivPEM), 0600); err != nil {
		return nil, nil, fmt.Errorf("failed to save CA private key: %w", err)
	}
	logger.Logf("Saved new CA private key to %s", caKeyPath)

	return caCertDER, caPriv, nil
}

// GenerateServerCert generates a server certificate signed by the CA.
func GenerateServerCert(caCertDER []byte, caPriv *ecdsa.PrivateKey) ([]byte, *ecdsa.PrivateKey, error) {
	caCert, err := x509.ParseCertificate(caCertDER)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to parse CA certificate: %w", err)
	}

	priv, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to generate server private key: %w", err)
	}

	serialNumber, err := generateRandomSerialNumber()
	if err != nil {
		return nil, nil, fmt.Errorf("failed to generate server certificate serial number: %w", err)
	}

	template := x509.Certificate{
		SerialNumber: serialNumber,
		Subject: pkix.Name{
			CommonName: "localhost",
		},
		NotBefore:   time.Now(),
		NotAfter:    time.Now().AddDate(1, 0, 0),
		ExtKeyUsage: []x509.ExtKeyUsage{x509.ExtKeyUsageServerAuth},
		KeyUsage:    x509.KeyUsageDigitalSignature,
		IPAddresses: []net.IP{net.ParseIP("127.0.0.1")},
		DNSNames:    []string{"localhost"},
	}

	derBytes, err := x509.CreateCertificate(rand.Reader, &template, caCert, &priv.PublicKey, caPriv)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to create server certificate: %w", err)
	}

	return derBytes, priv, nil
}

// LoadOrCreateServerCert loads an existing server certificate and key, or generates new ones.
func LoadOrCreateServerCert(serverCertPath, serverKeyPath string, caCertDER []byte, caPriv *ecdsa.PrivateKey) ([]byte, *ecdsa.PrivateKey, error) {
	// Try to load existing server cert and key
	serverCertPEM_bytes, err := os.ReadFile(serverCertPath)
	if err == nil {
		serverKeyPEM_bytes, err := os.ReadFile(serverKeyPath)
		if err == nil {
			// Both exist, parse them
			serverCert, err := tls.X509KeyPair(serverCertPEM_bytes, serverKeyPEM_bytes)
			if err != nil {
				return nil, nil, fmt.Errorf("failed to parse server X509 key pair from %s and %s: %w", serverCertPath, serverKeyPath, err)
			}
			parsedCert, err := x509.ParseCertificate(serverCert.Certificate[0])
			if err != nil {
				return nil, nil, fmt.Errorf("failed to parse server certificate from %s: %w", serverCertPath, err)
			}

			// Also parse the private key explicitly to return *ecdsa.PrivateKey
			block, _ := pem.Decode(serverKeyPEM_bytes)
			if block == nil || block.Type != "EC PRIVATE KEY" {
				return nil, nil, fmt.Errorf("failed to decode PEM block from %s", serverKeyPath)
			}
			priv, err := x509.ParseECPrivateKey(block.Bytes)
			if err != nil {
				return nil, nil, fmt.Errorf("failed to parse server private key from %s: %w", serverKeyPath, err)
			}

			logger.Logf("Loaded existing server certificate from %s and %s. Subject: %s, Issuer: %s, Serial: %s, NotBefore: %s, NotAfter: %s",
				serverCertPath, serverKeyPath, parsedCert.Subject, parsedCert.Issuer, parsedCert.SerialNumber, parsedCert.NotBefore.Format(time.RFC3339), parsedCert.NotAfter.Format(time.RFC3339))
			return parsedCert.Raw, priv, nil
		}
	}

	// If not found or error, generate new server cert
	logger.Logf("Generating new server certificate and key.")
	serverCertDER, serverPriv, err := GenerateServerCert(caCertDER, caPriv)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to generate new server cert: %w", err)
	}

	// Atomically save new server cert and key
	serverCertPEM := EncodeCertToPEM(serverCertDER)
	serverPrivPEM, err := EncodePrivKeyToPEM(serverPriv)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to encode server private key to PEM: %w", err)
	}

	if err := pathutil.AtomicWriteFile(serverCertPath, []byte(serverCertPEM), 0644); err != nil {
		return nil, nil, fmt.Errorf("failed to save server certificate: %w", err)
	}
	parsedServerCert, _ := x509.ParseCertificate(serverCertDER)
	logger.Logf("Saved new server certificate to %s. Subject: %s, Issuer: %s, Serial: %s, NotBefore: %s, NotAfter: %s",
		serverCertPath, parsedServerCert.Subject, parsedServerCert.Issuer, parsedServerCert.SerialNumber, parsedServerCert.NotBefore.Format(time.RFC3339), parsedServerCert.NotAfter.Format(time.RFC3339))

	if err := pathutil.AtomicWriteFile(serverKeyPath, []byte(serverPrivPEM), 0600); err != nil {
		return nil, nil, fmt.Errorf("failed to save server private key: %w", err)
	}
	logger.Logf("Saved new server private key to %s", serverKeyPath)

	return serverCertDER, serverPriv, nil
}

// SignClientPublicKey signs a PEM-encoded public key and returns a client certificate.
func SignClientPublicKey(caCertDER []byte, caPriv *ecdsa.PrivateKey, agentID string, pubKeyPEM string) ([]byte, error) {
	caCert, err := x509.ParseCertificate(caCertDER)
	if err != nil {
		return nil, fmt.Errorf("failed to parse CA certificate: %w", err)
	}

	block, _ := pem.Decode([]byte(pubKeyPEM))
	if block == nil || block.Type != "PUBLIC KEY" || len(block.Bytes) == 0 {
		return nil, &PublicKeyError{Message: "invalid or empty PEM block containing public key"}
	}

	pub, err := x509.ParsePKIXPublicKey(block.Bytes)
	if err != nil {
		return nil, &PublicKeyError{Message: fmt.Sprintf("failed to parse public key: %v", err)}
	}

	serialNumber, err := generateRandomSerialNumber()
	if err != nil {
		return nil, fmt.Errorf("failed to generate client certificate serial number: %w", err)
	}

	template := x509.Certificate{
		SerialNumber: serialNumber,
		Subject: pkix.Name{
			CommonName: agentID,
		},
		NotBefore:   time.Now(),
		NotAfter:    time.Now().AddDate(1, 0, 0),
		ExtKeyUsage: []x509.ExtKeyUsage{x509.ExtKeyUsageClientAuth},
		KeyUsage:    x509.KeyUsageDigitalSignature,
	}

	derBytes, err := x509.CreateCertificate(rand.Reader, &template, caCert, pub, caPriv)
	if err != nil {
		return nil, fmt.Errorf("failed to create client certificate: %w", err)
	}

	logger.Logf("Signed client certificate for agent ID: %s. Subject: %s, Issuer: %s, Serial: %s, NotBefore: %s, NotAfter: %s",
		agentID, template.Subject, caCert.Subject, template.SerialNumber, template.NotBefore.Format(time.RFC3339), template.NotAfter.Format(time.RFC3339))
	return derBytes, nil
}

// EncodeCertToPEM encodes a DER-encoded certificate to PEM.
func EncodeCertToPEM(derBytes []byte) string {
	return string(pem.EncodeToMemory(&pem.Block{Type: "CERTIFICATE", Bytes: derBytes}))
}

// EncodePrivKeyToPEM encodes an ECDSA private key to PEM.
func EncodePrivKeyToPEM(priv *ecdsa.PrivateKey) (string, error) {
	derBytes, err := x509.MarshalECPrivateKey(priv)
	if err != nil {
		return "", fmt.Errorf("failed to marshal EC private key: %w", err)
	}
	return string(pem.EncodeToMemory(&pem.Block{Type: "EC PRIVATE KEY", Bytes: derBytes})), nil
}

// MarshalPublicKey marshals a public key to ASN.1 DER form.
func MarshalPublicKey(pub interface{}) ([]byte, error) {
	return x509.MarshalPKIXPublicKey(pub)
}

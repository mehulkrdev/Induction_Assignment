use p256::ecdsa::{SigningKey, VerifyingKey};
use p256::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
use rand_core::OsRng;
use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use tokio::fs;
use std::io;

use enrollment_agent_logger::log_entry;


#[derive(Error, Debug)]
pub enum AgentError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Enrollment failed: {0}")]
    Enrollment(String),

    #[error("Server returned error: {status} - {body}")]
    ServerError {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("Generic error: {0}")]
    Generic(String),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

impl From<String> for AgentError {
    fn from(s: String) -> Self {
        AgentError::Generic(s)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct EnrollmentRequest {
    pub enrollment_token: String,
    pub agent_id: String,
    pub public_key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct EnrollmentResponse {
    pub status: String,
    pub certificate: Option<String>,
}

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait Discovery {
    fn discover_server(&self) -> Result<String, String>;
}

#[cfg_attr(test, automock)]
pub trait EnrollmentClient {
    fn enroll(&self, request: EnrollmentRequest) -> Result<EnrollmentResponse, String>;
}

pub struct CACertConfig {
    pub cert_path: PathBuf,
    pub expected_fingerprint: Option<String>,
}

pub struct Agent {
    pub agent_id: String,
    pub server_url: String,
    pub mtls_url: String,
    pub certs_path: PathBuf,
    pub ca_cert_config: Option<CACertConfig>,
}

#[cfg(test)]
pub struct AgentTestConfig {
    pub certs_path: Option<PathBuf>,
    pub ca_cert_config: Option<CACertConfig>,
}

impl Agent {
    pub fn new(agent_id: &str, server_url: &str) -> Self {
        // NOTE: We don't change the return type to Result to avoid breaking API changes.
        // Validation and mTLS URL derivation now happen via build_endpoint_url during use.
        Self {
            agent_id: agent_id.to_string(),
            server_url: server_url.to_string(),
            mtls_url: String::new(), // Deprecated but kept for field compatibility
            certs_path: PathBuf::from("."), // Default to current directory
            ca_cert_config: Some(CACertConfig {
                cert_path: PathBuf::from(".").join("ca.crt"), // Default CA cert path
                expected_fingerprint: None,
            }),
        }
    }

    #[cfg(test)]
    pub fn new_with_config(agent_id: &str, server_url: &str, mut config: AgentTestConfig) -> Self {
        let certs_path = config.certs_path.take().unwrap_or_else(|| PathBuf::from("."));
        Self {
            agent_id: agent_id.to_string(),
            server_url: server_url.to_string(),
            mtls_url: String::new(), // Deprecated but kept for field compatibility
            certs_path: certs_path.clone(),
            ca_cert_config: Some(config.ca_cert_config.take().unwrap_or_else(|| CACertConfig {
                cert_path: certs_path.join("ca.crt"),
                expected_fingerprint: None,
            })),
        }
    }

    /// Centralized URL construction and validation logic.
    /// Rejects non-8443 ports and uses explicit path construction.
    fn build_endpoint_url(&self, is_mtls: bool, endpoint: &str) -> Result<url::Url, AgentError> {
        let mut url = url::Url::parse(&self.server_url)?;

        // Validate port
        match url.port() {
            Some(8443) => {
                if is_mtls {
                    url.set_port(Some(8444)).map_err(|_| {
                        AgentError::InvalidUrl("Failed to set mTLS port".to_string())
                    })?;
                }
            }
            Some(p) => {
                return Err(AgentError::InvalidUrl(format!(
                    "Unexpected port: {}. Enrollment server must use port 8443.",
                    p
                )));
            }
            None => {
                return Err(AgentError::InvalidUrl(
                    "Port missing in server URL. Port 8443 is required.".to_string(),
                ));
            }
        }

        // Explicit path construction using path_segments_mut
        url.path_segments_mut()
            .map_err(|_| AgentError::InvalidUrl("URL cannot be a base".to_string()))?
            .clear()
            .push(endpoint);

        Ok(url)
    }

    pub async fn enroll(&self, token: &str) -> Result<(), AgentError> {
        log_entry!("Starting enrollment for agent: {}", self.agent_id);

        // 1. Generate ECDSA P-256 keypair
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);

        let priv_key_pem = signing_key
            .to_pkcs8_pem(LineEnding::LF)
            .map_err(|e| AgentError::Security(format!("Failed to encode private key: {}", e)))?;
        let pub_key_pem = verifying_key
            .to_public_key_pem(LineEnding::LF)
            .map_err(|e| AgentError::Security(format!("Failed to encode public key: {}", e)))?;

        // 2. Prepare enrollment request
        let request = EnrollmentRequest {
            enrollment_token: token.to_string(),
            agent_id: self.agent_id.clone(),
            public_key: pub_key_pem.to_string(),
        };

        // 3. Send request over HTTPS
        let mut cb = reqwest::Client::builder().use_rustls_tls();

        let ca_cert = {
            let config = self.ca_cert_config.as_ref().unwrap(); // ca_cert_config is always Some now
            let ca_cert_pem = fs::read(&config.cert_path).await?;
            if let Some(expected_fingerprint) = &config.expected_fingerprint {
                let actual_fingerprint = calculate_sha256(&ca_cert_pem);
                if actual_fingerprint != *expected_fingerprint {
                    return Err(AgentError::Security(format!(
                        "CA certificate fingerprint mismatch! Expected: {}, Actual: {}",
                        expected_fingerprint, actual_fingerprint
                    )));
                }
            }
            reqwest::Certificate::from_pem(&ca_cert_pem).map_err(|e| {
                AgentError::Security(format!(
                    "Failed to parse ca.crt from {}: {}",
                    config.cert_path.display(),
                    e
                ))
            })?
        };

        cb = cb.add_root_certificate(ca_cert);

        // Build the client for enrollment (port 8443)
        let client_enroll = cb.build()?;
        let url = self.build_endpoint_url(false, "enroll")?;
        let mut attempts = 0;
        let max_attempts = 3;
        let mut delay = std::time::Duration::from_secs(1);

        let res = loop {
            attempts += 1;
            match client_enroll.post(url.clone()).json(&request).send().await {
                Ok(res) => break Ok(res),
                Err(e) => {
                    log_entry!(
                        "WARNING: Enrollment request failed (attempt {}/{}) for agent {}: {}. Detailed error: {:?}",
                        attempts,
                        max_attempts,
                        self.agent_id,
                        e,
                        e // Log the full error for more details
                    );
                    if attempts >= max_attempts {
                        break Err(e);
                    }
                    tokio::time::sleep(delay).await;
                    delay *= 2; // Exponential backoff
                }
            }
        }?;

        if res.status() != reqwest::StatusCode::OK {
            let status = res.status();
            let error_body = res
                .text()
                .await
                .unwrap_or_else(|_| "<unavailable>".to_string());
            log_entry!(
                "ERROR: Server returned error status for enrollment: {} - Body: {}",
                status,
                error_body
            );
            return Err(AgentError::ServerError {
                status,
                body: error_body,
            });
        }

        let response: EnrollmentResponse = res.json().await?;

        if response.status != "success" {
            log_entry!("ERROR: Enrollment failed with status: {}", response.status);
            return Err(AgentError::Enrollment(response.status));
        }

        let cert_pem = response
            .certificate
            .ok_or_else(|| AgentError::Enrollment("No certificate received from server".to_string()))?;

        // 4. Persist keys and certificate
        let key_path = self.certs_path.join("agent.key");
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut options = std::fs::OpenOptions::new();
            options.create(true).write(true).truncate(true).mode(0o600);
            let mut file = options.open(&key_path).map_err(AgentError::Io)?;
            use std::io::Write;
            file.write_all(priv_key_pem.as_bytes()).map_err(AgentError::Io)?;
        }
        #[cfg(not(unix))]
        {
            fs::write(&key_path, priv_key_pem.as_bytes()).await?;
        }

        fs::write(self.certs_path.join("agent.crt"), cert_pem.as_bytes()).await?;

        log_entry!(
            "Enrollment successful. Certificate and key saved to {}.",
            self.certs_path.display()
        );
        Ok(())
    }

    pub async fn reconnect(&self) -> Result<String, AgentError> {
        log_entry!("Attempting mTLS reconnection for agent: {}", self.agent_id);

        let agent_key_path = self.certs_path.join("agent.key");
        let agent_crt_path = self.certs_path.join("agent.crt");

        if !agent_key_path.exists() || !agent_crt_path.exists() {
            log_entry!(
                "DEBUG: Identity files ({}, {}) not found in {}. Enroll first.",
                agent_key_path.display(),
                agent_crt_path.display(),
                self.certs_path.display()
            );
            return Err(AgentError::Security(
                "mTLS identity files not found. Please enroll first.".to_string(),
            ));
        }

        let priv_key_pem = fs::read_to_string(&agent_key_path).await?;
        let cert_pem = fs::read_to_string(&agent_crt_path).await?;

        let identity = reqwest::Identity::from_pem((cert_pem.clone() + "\n" + &priv_key_pem).as_bytes())
            .map_err(|e| {
                log_entry!("DEBUG: Failed to create identity from agent.key and agent.crt (mismatch or malformed): {:?}", e);
                AgentError::Security(
                    "Failed to establish mTLS identity. Check certificate validity.".to_string(),
                )
            })?;

        let mut cb = reqwest::Client::builder()
            .use_rustls_tls()
            .identity(identity);

        let ca_cert = {
            let config = self.ca_cert_config.as_ref().unwrap(); // ca_cert_config is always Some now
            let ca_cert_pem = fs::read(&config.cert_path).await?;
            if let Some(expected_fingerprint) = &config.expected_fingerprint {
                let actual_fingerprint = calculate_sha256(&ca_cert_pem);
                if actual_fingerprint != *expected_fingerprint {
                    return Err(AgentError::Security(format!(
                        "CA certificate fingerprint mismatch! Expected: {}, Actual: {}",
                        expected_fingerprint, actual_fingerprint
                    )));
                }
            }
            reqwest::Certificate::from_pem(&ca_cert_pem).map_err(|e| {
                log_entry!(
                    "DEBUG: Failed to parse ca.crt from {}: {:?}",
                    config.cert_path.display(),
                    e
                );
                AgentError::Security("Failed to parse CA certificate.".to_string())
            })?
        };

        cb = cb.add_root_certificate(ca_cert);

        // Build the client for reconnection (port 8444, mTLS identity already added)
        let client_reconnect = cb.build()?;
        let secure_url = self.build_endpoint_url(true, "secure")?;
        let mut attempts = 0;
        let max_attempts = 3;
        let mut delay = std::time::Duration::from_secs(1);

        let res = loop {
            attempts += 1;
            match client_reconnect.get(secure_url.clone()).send().await {
                Ok(res) => break Ok(res),
                Err(e) => {
                    log_entry!("WARNING: mTLS reconnection request failed (attempt {}/{}) for agent {}: {}. Detailed error: {:?}",
                               attempts, max_attempts, self.agent_id, e, e);
                    if attempts >= max_attempts {
                        break Err(e);
                    }
                    tokio::time::sleep(delay).await;
                    delay *= 2; // Exponential backoff
                }
            }
        }?;
        if res.status() != reqwest::StatusCode::OK {
            let status = res.status();
            let error_body = res
                .text()
                .await
                .unwrap_or_else(|_| "<unavailable>".to_string());
            log_entry!(
                "ERROR: mTLS Server returned error status: {} - Body: {}",
                status,
                error_body
            );
            return Err(AgentError::ServerError {
                status,
                body: error_body,
            });
        }

        let body = res.text().await?;
        log_entry!("mTLS reconnection successful: {}", body);
        Ok(body)
    }
}

fn calculate_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_construction_valid() {
        let agent = Agent::new("test-agent", "https://localhost:8443");
        
        let enroll_url = agent.build_endpoint_url(false, "enroll").unwrap();
        assert_eq!(enroll_url.as_str(), "https://localhost:8443/enroll");
        
        let secure_url = agent.build_endpoint_url(true, "secure").unwrap();
        assert_eq!(secure_url.as_str(), "https://localhost:8444/secure");
    }

    #[test]
    fn test_url_construction_with_trailing_slash() {
        let agent = Agent::new("test-agent", "https://localhost:8443/");
        
        let enroll_url = agent.build_endpoint_url(false, "enroll").unwrap();
        assert_eq!(enroll_url.as_str(), "https://localhost:8443/enroll");
        
        let secure_url = agent.build_endpoint_url(true, "secure").unwrap();
        assert_eq!(secure_url.as_str(), "https://localhost:8444/secure");
    }

    #[test]
    fn test_url_construction_with_path() {
        // Even if there's a path, build_endpoint_url should clear it
        let agent = Agent::new("test-agent", "https://localhost:8443/extra/path");
        
        let enroll_url = agent.build_endpoint_url(false, "enroll").unwrap();
        assert_eq!(enroll_url.as_str(), "https://localhost:8443/enroll");
        
        let secure_url = agent.build_endpoint_url(true, "secure").unwrap();
        assert_eq!(secure_url.as_str(), "https://localhost:8444/secure");
    }

    #[test]
    fn test_url_construction_invalid_port() {
        let agent = Agent::new("test-agent", "https://localhost:8080");
        let result = agent.build_endpoint_url(false, "enroll");
        assert!(matches!(result, Err(AgentError::InvalidUrl(_))));
        if let Err(AgentError::InvalidUrl(msg)) = result {
            assert!(msg.contains("Unexpected port: 8080"));
        }
    }

    #[test]
    fn test_url_construction_missing_port() {
        let agent = Agent::new("test-agent", "https://localhost");
        let result = agent.build_endpoint_url(false, "enroll");
        assert!(matches!(result, Err(AgentError::InvalidUrl(_))));
        if let Err(AgentError::InvalidUrl(msg)) = result {
            assert!(msg.contains("Port missing"));
        }
    }

    #[test]
    fn test_url_construction_malformed() {
        let agent = Agent::new("test-agent", "not a url");
        let result = agent.build_endpoint_url(false, "enroll");
        assert!(matches!(result, Err(AgentError::UrlParse(_))));
    }
}

use p256::ecdsa::{SigningKey, VerifyingKey};
use p256::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
use rand_core::OsRng;
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

pub struct Agent {
    pub agent_id: String,
    pub server_url: String,
    pub certs_path: PathBuf,
    pub ca_cert_path: Option<PathBuf>,
}

#[cfg(test)]
pub struct AgentTestConfig {
    pub certs_path: Option<PathBuf>,
    pub ca_cert_path: Option<PathBuf>,
}

impl Agent {
    pub fn new(agent_id: &str, server_url: &str) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            server_url: server_url.to_string(),
            certs_path: PathBuf::from("."), // Default to current directory
            ca_cert_path: None,
        }
    }

    #[cfg(test)]
    pub fn new_with_config(agent_id: &str, server_url: &str, config: AgentTestConfig) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            server_url: server_url.to_string(),
            certs_path: config.certs_path.unwrap_or_else(|| PathBuf::from(".")),
            ca_cert_path: config.ca_cert_path,
        }
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

        let ca_cert_path = self.ca_cert_path.clone().unwrap_or_else(|| self.certs_path.join("ca.crt"));
        if ca_cert_path.exists() {
            let ca_cert_pem = fs::read(&ca_cert_path).await?;
            let ca_cert = reqwest::Certificate::from_pem(&ca_cert_pem).map_err(|e| {
                AgentError::Security(format!(
                    "Failed to parse ca.crt from {}: {}",
                    ca_cert_path.display(),
                    e
                ))
            })?;

            cb = cb.add_root_certificate(ca_cert);
        } else {
            log_entry!("ERROR: ca.crt not found at {}. Cannot establish secure connection without CA certificate.", ca_cert_path.display());
            return Err(AgentError::Security(format!("ca.crt not found at {}. Cannot establish secure connection without CA certificate.", ca_cert_path.display())));
        }

        let client = cb.build()?;

        let url = format!("{}/enroll", self.server_url);

        let mut attempts = 0;
        let max_attempts = 3;
        let mut delay = std::time::Duration::from_secs(1);

        let res = loop {
            attempts += 1;
            match client.post(&url).json(&request).send().await {
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
            return Err(AgentError::Security(format!(
                "Identity files ({}, {}) not found in {}. Enroll first.",
                agent_key_path.display(),
                agent_crt_path.display(),
                self.certs_path.display()
            )));
        }

        let priv_key_pem = fs::read_to_string(&agent_key_path).await?;
        let cert_pem = fs::read_to_string(&agent_crt_path).await?;

        let identity = reqwest::Identity::from_pem((cert_pem.clone() + "\n" + &priv_key_pem).as_bytes())
            .map_err(|e| {
                log_entry!("ERROR: Failed to create identity from agent.key and agent.crt (mismatch or malformed): {}", e);
                AgentError::Security(format!("Failed to create identity: {}", e))
            })?;

        let mut cb = reqwest::Client::builder()
            .use_rustls_tls()
            .identity(identity);

        let ca_cert_path = self.ca_cert_path.clone().unwrap_or_else(|| self.certs_path.join("ca.crt"));
        if ca_cert_path.exists() {
            let ca_cert_pem = fs::read(&ca_cert_path).await?;
            let ca_cert = reqwest::Certificate::from_pem(&ca_cert_pem).map_err(|e| {
                AgentError::Security(format!(
                    "Failed to parse ca.crt from {}: {}",
                    ca_cert_path.display(),
                    e
                ))
            })?;

            cb = cb.add_root_certificate(ca_cert);
        } else {
            log_entry!("ERROR: ca.crt not found at {}. Cannot establish secure connection without CA certificate.", ca_cert_path.display());
            return Err(AgentError::Security(format!("ca.crt not found at {}. Cannot establish secure connection without CA certificate.", ca_cert_path.display())));
        }

        let client = cb.build()?;

        let m_tls_url = self.server_url.replace(":8443", ":8444");
        let secure_url = format!("{}/secure", m_tls_url);

        let mut attempts = 0;
        let max_attempts = 3;
        let mut delay = std::time::Duration::from_secs(1);

        let res = loop {
            attempts += 1;
            match client.get(&secure_url).send().await {
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

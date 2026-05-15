use p256::ecdsa::{SigningKey, VerifyingKey};
use p256::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::path::PathBuf;

#[macro_use]
extern crate enrollment_agent_logger;

#[cfg(test)]
use crate::test_helpers::AgentTestConfig; // Only used in test configuration

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
}

impl Agent {
    pub fn new(agent_id: &str, server_url: &str) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            server_url: server_url.to_string(),
            certs_path: PathBuf::from("."), // Default to current directory
        }
    }

    #[cfg(test)]
    pub fn new_with_config(agent_id: &str, server_url: &str, config: AgentTestConfig) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            server_url: server_url.to_string(),
            certs_path: config.certs_path.unwrap_or_else(|| PathBuf::from(".")), // Use provided path or default
        }
    }

    pub async fn enroll(&self, token: &str) -> Result<(), String> {
        log_entry!("Starting enrollment for agent: {}", self.agent_id);

        // 1. Generate ECDSA P-256 keypair
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);

        let priv_key_pem = signing_key
            .to_pkcs8_pem(LineEnding::LF)
            .map_err(|e| format!("Failed to encode private key: {}", e))?;
        let pub_key_pem = verifying_key
            .to_public_key_pem(LineEnding::LF)
            .map_err(|e| format!("Failed to encode public key: {}", e))?;

        // 2. Prepare enrollment request
        let request = EnrollmentRequest {
            enrollment_token: token.to_string(),
            agent_id: self.agent_id.clone(),
            public_key: pub_key_pem.to_string(),
        };

        // 3. Send request over HTTPS
        let mut cb = reqwest::Client::builder().use_rustls_tls();

        let ca_cert_path = self.certs_path.join("ca.crt");
        if ca_cert_path.exists() {
            let ca_cert_pem = fs::read(&ca_cert_path).map_err(|e| {
                format!(
                    "Failed to read ca.crt from {}: {}",
                    ca_cert_path.display(),
                    e
                )
            })?;
            let ca_cert = reqwest::Certificate::from_pem(&ca_cert_pem).map_err(|e| {
                format!(
                    "Failed to parse ca.crt from {}: {}",
                    ca_cert_path.display(),
                    e
                )
            })?;
            cb = cb.add_root_certificate(ca_cert);
        } else {
            log_entry!("ERROR: ca.crt not found at {}. Cannot establish secure connection without CA certificate.", ca_cert_path.display());
            return Err(format!("ca.crt not found at {}. Cannot establish secure connection without CA certificate.", ca_cert_path.display()));
        }

        let client = cb.build().map_err(|e| {
            log_entry!("ERROR: Failed to build reqwest client: {}", e);
            format!("Failed to build reqwest client: {}", e)
        })?;

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
                        "WARNING: Enrollment request failed (attempt {}/{}) for agent {}: {}",
                        attempts,
                        max_attempts,
                        self.agent_id,
                        e
                    );
                    if attempts >= max_attempts {
                        break Err(format!(
                            "Enrollment request failed after {} attempts: {}",
                            max_attempts, e
                        ));
                    }
                    tokio::time::sleep(delay).await;
                    delay *= 2; // Exponential backoff
                }
            }
        }
        .map_err(|e| e)?;

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
            return Err(format!(
                "Server returned error: {} - {}",
                status, error_body
            ));
        }

        let response: EnrollmentResponse = res.json().await.map_err(|e| {
            log_entry!("ERROR: Failed to parse enrollment response: {}", e);
            format!("Failed to parse enrollment response: {}", e)
        })?;

        if response.status != "success" {
            log_entry!("ERROR: Enrollment failed with status: {}", response.status);
            return Err(format!("Enrollment failed: {}", response.status));
        }

        let cert_pem = response
            .certificate
            .ok_or("No certificate received from server")?;

        // 4. Persist keys and certificate
        fs::write(self.certs_path.join("agent.key"), priv_key_pem.as_bytes())
            .await
            .map_err(|e| {
                format!(
                    "Failed to save agent.key to {}: {}",
                    self.certs_path.display(),
                    e
                )
            })?;
        fs::write(self.certs_path.join("agent.crt"), cert_pem.as_bytes())
            .await
            .map_err(|e| {
                format!(
                    "Failed to save agent.crt to {}: {}",
                    self.certs_path.display(),
                    e
                )
            })?;

        log_entry!(
            "Enrollment successful. Certificate and key saved to {}.",
            self.certs_path.display()
        );
        Ok(())
    }

    pub async fn reconnect(&self) -> Result<String, String> {
        log_entry!("Attempting mTLS reconnection for agent: {}", self.agent_id);

        let agent_key_path = self.certs_path.join("agent.key");
        let agent_crt_path = self.certs_path.join("agent.crt");

        if !agent_key_path.exists() || !agent_crt_path.exists() {
            return Err(format!(
                "Identity files ({}, {}) not found in {}. Enroll first.",
                agent_key_path.display(),
                agent_crt_path.display(),
                self.certs_path.display()
            ));
        }

        let priv_key_pem = fs::read_to_string(&agent_key_path).await.map_err(|e| {
            log_entry!(
                "ERROR: Failed to read agent.key from {}: {}",
                agent_key_path.display(),
                e
            );
            format!(
                "Failed to read agent.key from {}: {}",
                agent_key_path.display(),
                e
            )
        })?;
        let cert_pem = fs::read_to_string(&agent_crt_path).await.map_err(|e| {
            log_entry!(
                "ERROR: Failed to read agent.crt from {}: {}",
                agent_crt_path.display(),
                e
            );
            format!(
                "Failed to read agent.crt from {}: {}",
                agent_crt_path.display(),
                e
            )
        })?;

        let identity = reqwest::Identity::from_pem((cert_pem.clone() + "\n" + &priv_key_pem).as_bytes())
            .map_err(|e| {
                log_entry!("ERROR: Failed to create identity from agent.key and agent.crt (mismatch or malformed): {}", e);
                format!("Failed to create identity: {}", e)
            })?;

        let mut cb = reqwest::Client::builder()
            .use_rustls_tls()
            .identity(identity);

        let ca_cert_path = self.certs_path.join("ca.crt");
        if ca_cert_path.exists() {
            let ca_cert_pem = fs::read(&ca_cert_path).await.map_err(|e| {
                log_entry!(
                    "ERROR: Failed to read ca.crt from {}: {}",
                    ca_cert_path.display(),
                    e
                );
                format!(
                    "Failed to read ca.crt from {}: {}",
                    ca_cert_path.display(),
                    e
                )
            })?;
            let ca_cert = reqwest::Certificate::from_pem(&ca_cert_pem).map_err(|e| {
                log_entry!(
                    "ERROR: Failed to parse ca.crt from {}: {}",
                    ca_cert_path.display(),
                    e
                );
                format!(
                    "Failed to parse ca.crt from {}: {}",
                    ca_cert_path.display(),
                    e
                )
            })?;
            cb = cb.add_root_certificate(ca_cert);
        } else {
            log_entry!("ERROR: ca.crt not found at {}. Cannot establish secure connection without CA certificate.", ca_cert_path.display());
            return Err(format!("ca.crt not found at {}. Cannot establish secure connection without CA certificate.", ca_cert_path.display()));
        }

        let client = cb.build().map_err(|e| {
            log_entry!("ERROR: Failed to build reqwest client for mTLS: {}", e);
            format!("Failed to build reqwest client for mTLS: {}", e)
        })?;

        let m_tls_url = self.server_url.replace("8443", "8444");
        let secure_url = format!("{}/secure", m_tls_url);

        let mut attempts = 0;
        let max_attempts = 3;
        let mut delay = std::time::Duration::from_secs(1);

        let res = loop {
            attempts += 1;
            match client.get(&secure_url)
                .send()
                .await {
                Ok(res) => break Ok(res),
                Err(e) => {
                    log_entry!("WARNING: mTLS reconnection request failed (attempt {}/{}) for agent {}: {}", attempts, max_attempts, self.agent_id, e);
                    if attempts >= max_attempts {
                        break Err(format!("mTLS reconnection request failed after {} attempts: {}", max_attempts, e));
                    }
                    tokio::time::sleep(delay).await;
                    delay *= 2; // Exponential backoff
                }
            }
        }.map_err(|e| e)?;

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
            return Err(format!(
                "Server returned error: {} - {}",
                status, error_body
            ));
        }

        let body = res.text().await.map_err(|e| {
            log_entry!("ERROR: Failed to read mTLS response body: {}", e);
            format!("Failed to read mTLS response body: {}", e)
        })?;
        log_entry!("mTLS reconnection successful: {}", body);
        Ok(body)
    }
}

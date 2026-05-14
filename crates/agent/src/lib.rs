use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use p256::ecdsa::{SigningKey, VerifyingKey};
use p256::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
use rand_core::OsRng;

#[macro_use]
extern crate enrollment_agent_logger;

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
}

impl Agent {
    pub fn new(agent_id: &str, server_url: &str) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            server_url: server_url.to_string(),
        }
    }

    pub async fn enroll(&self, token: &str) -> Result<(), String> {
        log_entry!("Starting enrollment for agent: {}", self.agent_id);

        // 1. Generate ECDSA P-256 keypair
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);

        let priv_key_pem = signing_key.to_pkcs8_pem(LineEnding::LF)
            .map_err(|e| format!("Failed to encode private key: {}", e))?;
        let pub_key_pem = verifying_key.to_public_key_pem(LineEnding::LF)
            .map_err(|e| format!("Failed to encode public key: {}", e))?;

        // 2. Prepare enrollment request
        let request = EnrollmentRequest {
            enrollment_token: token.to_string(),
            agent_id: self.agent_id.clone(),
            public_key: pub_key_pem.to_string(),
        };

        // 3. Send request over HTTPS
        let mut cb = reqwest::Client::builder()
            .use_rustls_tls();
        
        let ca_cert_path = Path::new("ca.crt");
        if ca_cert_path.exists() {
            let ca_cert_pem = fs::read(ca_cert_path)
                .map_err(|e| format!("Failed to read ca.crt: {}", e))?;
            let ca_cert = reqwest::Certificate::from_pem(&ca_cert_pem)
                .map_err(|e| format!("Failed to parse ca.crt: {}", e))?;
            cb = cb.add_root_certificate(ca_cert);
        } else {
            log_entry!("ERROR: ca.crt not found. Cannot establish secure connection without CA certificate.");
            return Err("ca.crt not found. Cannot establish secure connection without CA certificate.".to_string());
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
            match client.post(&url)
                .json(&request)
                .send()
                .await {
                Ok(res) => break Ok(res),
                Err(e) => {
                    log_entry!("WARNING: Enrollment request failed (attempt {}/{}) for agent {}: {}", attempts, max_attempts, self.agent_id, e);
                    if attempts >= max_attempts {
                        break Err(format!("Enrollment request failed after {} attempts: {}", max_attempts, e));
                    }
                    tokio::time::sleep(delay).await;
                    delay *= 2; // Exponential backoff
                }
            }
        }.map_err(|e| e)?;

        if res.status() != reqwest::StatusCode::OK {
            let status = res.status();
            let error_body = res.text().await.unwrap_or_else(|_| "<unavailable>".to_string());
            log_entry!("ERROR: Server returned error status for enrollment: {} - Body: {}", status, error_body);
            return Err(format!("Server returned error: {} - {}", status, error_body));
        }

        let response: EnrollmentResponse = res.json().await
            .map_err(|e| {
                log_entry!("ERROR: Failed to parse enrollment response: {}", e);
                format!("Failed to parse enrollment response: {}", e)
            })?;

        if response.status != "success" {
            log_entry!("ERROR: Enrollment failed with status: {}", response.status);
            return Err(format!("Enrollment failed: {}", response.status));
        }

        let cert_pem = response.certificate.ok_or("No certificate received from server")?;

        // 4. Persist keys and certificate
        fs::write("agent.key", priv_key_pem.as_bytes())
            .map_err(|e| format!("Failed to save agent.key: {}", e))?;
        fs::write("agent.crt", cert_pem.as_bytes())
            .map_err(|e| format!("Failed to save agent.crt: {}", e))?;

        log_entry!("Enrollment successful. Certificate and key saved.");
        Ok(())
    }

    pub async fn reconnect(&self) -> Result<String, String> {
        log_entry!("Attempting mTLS reconnection for agent: {}", self.agent_id);

        if !Path::new("agent.key").exists() || !Path::new("agent.crt").exists() {
            return Err("Identity files (agent.key or agent.crt) not found. Enroll first.".to_string());
        }

        let priv_key_pem = fs::read_to_string("agent.key")
            .map_err(|e| {
                log_entry!("ERROR: Failed to read agent.key: {}", e);
                format!("Failed to read agent.key: {}", e)
            })?;
        let cert_pem = fs::read_to_string("agent.crt")
            .map_err(|e| {
                log_entry!("ERROR: Failed to read agent.crt: {}", e);
                format!("Failed to read agent.crt: {}", e)
            })?;

        let identity = reqwest::Identity::from_pem((cert_pem.clone() + "\n" + &priv_key_pem).as_bytes())
            .map_err(|e| {
                log_entry!("ERROR: Failed to create identity from agent.key and agent.crt (mismatch or malformed): {}", e);
                format!("Failed to create identity: {}", e)
            })?;

        let mut cb = reqwest::Client::builder()
            .use_rustls_tls()
            .identity(identity);

        let ca_cert_path = Path::new("ca.crt");
        if ca_cert_path.exists() {
            let ca_cert_pem = fs::read(ca_cert_path)
                .map_err(|e| {
                    log_entry!("ERROR: Failed to read ca.crt: {}", e);
                    format!("Failed to read ca.crt: {}", e)
                })?;
            let ca_cert = reqwest::Certificate::from_pem(&ca_cert_pem)
                .map_err(|e| {
                    log_entry!("ERROR: Failed to parse ca.crt: {}", e);
                    format!("Failed to parse ca.crt: {}", e)
                })?;
            cb = cb.add_root_certificate(ca_cert);
        } else {
            log_entry!("ERROR: ca.crt not found. Cannot establish secure connection without CA certificate.");
            return Err("ca.crt not found. Cannot establish secure connection without CA certificate.".to_string());
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
            let error_body = res.text().await.unwrap_or_else(|_| "<unavailable>".to_string());
            log_entry!("ERROR: mTLS Server returned error status: {} - Body: {}", status, error_body);
            return Err(format!("Server returned error: {} - {}", status, error_body));
        }

        let body = res.text().await
            .map_err(|e| {
                log_entry!("ERROR: Failed to read mTLS response body: {}", e);
                format!("Failed to read mTLS response body: {}", e)
            })?;
        log_entry!("mTLS reconnection successful: {}", body);
        Ok(body)
    }
}

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use p256::ecdsa::{SigningKey, VerifyingKey};
use p256::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
use rand_core::OsRng;

#[macro_use]
#[path = "../../logger/client/logger.rs"]
pub mod logger;

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
        
        // Try to load CA cert if it exists
        if Path::new("ca.crt").exists() {
            let ca_cert_pem = fs::read("ca.crt").map_err(|e| format!("Failed to read ca.crt: {}", e))?;
            let ca_cert = reqwest::Certificate::from_pem(&ca_cert_pem)
                .map_err(|e| format!("Failed to parse ca.crt: {}", e))?;
            cb = cb.add_root_certificate(ca_cert);
        } else {
            // In a production environment, ca.crt should always be present and trusted.
            // Using `danger_accept_invalid_certs(true)` is a security risk and should be avoided.
            return Err("ca.crt not found. Cannot establish secure connection without CA certificate.".to_string());
        }

        let client = cb.build().map_err(|e| format!("Failed to build reqwest client: {}", e))?;
        
        let url = format!("{}/enroll", self.server_url);
        let res = client.post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Enrollment request failed: {}", e))?;

        if res.status() != reqwest::StatusCode::OK {
            return Err(format!("Server returned error: {}", res.status()));
        }

        let response: EnrollmentResponse = res.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if response.status != "success" {
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

        let priv_key_pem = fs::read_to_string("agent.key").map_err(|e| e.to_string())?;
        let cert_pem = fs::read_to_string("agent.crt").map_err(|e| e.to_string())?;

        let identity = reqwest::Identity::from_pem((cert_pem + "\n" + &priv_key_pem).as_bytes())
            .map_err(|e| format!("Failed to create identity: {}", e))?;

        let mut cb = reqwest::Client::builder()
            .use_rustls_tls()
            .identity(identity);

        if Path::new("ca.crt").exists() {
            let ca_cert_pem = fs::read("ca.crt").map_err(|e| e.to_string())?;
            let ca_cert = reqwest::Certificate::from_pem(&ca_cert_pem).map_err(|e| e.to_string())?;
            cb = cb.add_root_certificate(ca_cert);
        } else {
            // In a production environment, ca.crt should always be present and trusted.
            // Using `danger_accept_invalid_certs(true)` is a security risk and should be avoided.
            return Err("ca.crt not found. Cannot establish secure connection without CA certificate.".to_string());
        }

        let client = cb.build().map_err(|e| e.to_string())?;
        
        let m_tls_url = self.server_url.replace("8443", "8444");
        let secure_url = format!("{}/secure", m_tls_url);
        
        let res = client.get(&secure_url)
            .send()
            .await
            .map_err(|e| format!("mTLS request failed: {}", e))?;

        if res.status() != reqwest::StatusCode::OK {
            return Err(format!("mTLS Server returned error: {}", res.status()));
        }

        let body = res.text().await.map_err(|e| e.to_string())?;
        log_entry!("mTLS reconnection successful: {}", body);
        Ok(body)
    }
}

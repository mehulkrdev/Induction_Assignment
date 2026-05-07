use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct EnrollmentRequest {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpStream;

    #[tokio::test]
    async fn test_agent_connection_to_server_port_8443() {
        let addr = "127.0.0.1:8443";
        let stream = TcpStream::connect(addr).await;
        assert!(stream.is_ok(), "Agent failed to connect to the server on port 8443");
    }

    #[tokio::test]
    async fn test_enrollment_request_fail() {
        // This is a placeholder for a failing enrollment request test
        assert!(false, "Enrollment request flow not yet tested");
    }
}

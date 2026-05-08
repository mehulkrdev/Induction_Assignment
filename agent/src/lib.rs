use serde::{Deserialize, Serialize};
#[macro_use]
#[path = "../../logger/client/logger.rs"]
pub mod logger;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)] // this statements help to automatically creates necessary function needed to safely transfer owenrhip of the object of this struct
pub struct EnrollmentRequest {
    pub enrollment_token: String,
    pub agent_id: String,
    pub public_key: String,
}// when agent erolls with server. it sends afent_id, public_key as payload

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct EnrollmentResponse {
    pub status: String,
    pub certificate: Option<String>, // std::optional<std::string>
}

#[cfg(test)]
use mockall::automock; // only import this during testing.

#[cfg_attr(test, automock)]
pub trait Discovery {
    fn discover_server(&self) -> Result<String, String>;
}
/*class Discovery {
public:
    virtual Result discoverServer() = 0;
};
fn discover_server(&self) -> discoverServer() const
&self: borrow object, don\'t take ownership, read-only access
Result<String, String>; -> std::expected<std::string, std::string>
*/

#[cfg_attr(test, automock)] // "When compiling tests, auto-generate a mock version." (Rust feature)
pub trait EnrollmentClient { // "Something capable of sending enrollment request."
    fn enroll(&self, request: EnrollmentRequest) -> Result<EnrollmentResponse, String>;
}
/* notice how EnrollmentRequest is used intead of & EnrollmentRequest meaning the ownership 
moves into function.

WHY Move Ownership?

Usually because:

request is consumed
no need to retain caller ownership
avoids unnecessary copies

Very common Rust optimization pattern.

*/
#[cfg(test)]
mod tests {
    use super::*;
    use reqwest;
    use crate::logger;
    use std::fs::File;

    // Helper to create a dummy log file for tests
    fn setup_test_logger() -> File {
        let path = std::env::temp_dir().join(format!("test_agent_log_{}.log", chrono::Local::now().format("%Y%m%d%H%M%S")));
        File::create(&path).expect("Failed to create test log file")
    }

    #[tokio::test]
    async fn test_agent_enrollment_post_success() {
        let _guard = logger::set_log_file_for_tests(setup_test_logger());
        log_entry!("Starting test_agent_enrollment_post_success");

        let client = reqwest::Client::new();
        let request = EnrollmentRequest {
            enrollment_token: "valid-token".to_string(),
            agent_id: "agent-1".to_string(),
            public_key: "key-data".to_string(),
        };

        let res = client.post("http://localhost:8443/enroll")
            .json(&request)
            .send()
            .await;
        
        assert!(res.is_ok(), "Agent failed to send POST request to the server");
        let res = res.unwrap();
        log_entry!("Server response status: {}", res.status());
        assert_eq!(res.status(), reqwest::StatusCode::OK, "Server did not return 200 OK");
    }

    #[tokio::test]
    async fn test_agent_enrollment_invalid_token_401() {
        let _guard = logger::set_log_file_for_tests(setup_test_logger());
        log_entry!("Starting test_agent_enrollment_invalid_token_401");

        let client = reqwest::Client::new();
        let request = EnrollmentRequest {
            enrollment_token: "invalid-token".to_string(),
            agent_id: "agent-1".to_string(),
            public_key: "key-data".to_string(),
        };

        let res = client.post("http://localhost:8443/enroll")
            .json(&request)
            .send()
            .await;
        
        assert!(res.is_ok());
        let res = res.unwrap();
        log_entry!("Server response status: {}", res.status());
        assert_eq!(res.status(), reqwest::StatusCode::UNAUTHORIZED, "Server did not return 401 Unauthorized");
    }

    #[tokio::test]
    async fn test_agent_enrollment_malformed_request_400() {
        let _guard = logger::set_log_file_for_tests(setup_test_logger());
        log_entry!("Starting test_agent_enrollment_malformed_request_400");

        let client = reqwest::Client::new();
        // Sending empty agent_id which should trigger 400
        let request = EnrollmentRequest {
            enrollment_token: "valid-token".to_string(),
            agent_id: "".to_string(),
            public_key: "key-data".to_string(),
        };

        let res = client.post("http://localhost:8443/enroll")
            .json(&request)
            .send()
            .await;
        
        assert!(res.is_ok());
        let res = res.unwrap();
        log_entry!("Server response status: {}", res.status());
        assert_eq!(res.status(), reqwest::StatusCode::BAD_REQUEST, "Server did not return 400 Bad Request");
    }
}

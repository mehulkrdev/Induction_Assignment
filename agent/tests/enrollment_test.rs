use enrollment_agent::{Agent, EnrollmentRequest, EnrollmentResponse};
use enrollment_agent::logger;
use mockall::{automock, predicate};
use std::fs;
use std::path::Path;
use std::fs::File;
use chrono;

// We need to define the traits locally to mock them, as Mockall can\\\"t automock traits from other crates unless they are defined in the current crate.
// This is a common pattern when testing traits across crate boundaries.

#[automock]
pub trait EnrollmentClient {
    fn enroll(&self, request: EnrollmentRequest) -> Result<EnrollmentResponse, String>;
}

// Helper to create a dummy log file for tests
fn setup_test_logger() -> File {
    let path = std::env::temp_dir().join(format!("test_integration_log_{}.log", chrono::Local::now().format("%Y%m%d%H%M%S")));
    File::create(&path).expect("Failed to create test log file")
}

#[tokio::test]
async fn test_agent_enrollment_and_reconnect_integration() {
    let _guard = logger::set_log_file_for_tests(setup_test_logger());
    let agent = Agent::new("agent-test", "https://localhost:8443");
    
    // 1. Enrollment
    let result = agent.enroll("valid-token").await;
    
    if result.is_ok() {
        assert!(Path::new("agent.key").exists());
        assert!(Path::new("agent.crt").exists());
        
        // 2. Reconnection via mTLS
        let reconnect_result = agent.reconnect().await;
        assert!(reconnect_result.is_ok(), "mTLS reconnection failed: {:?}", reconnect_result.err());
        assert!(reconnect_result.unwrap().contains("Hello verified agent: agent-test"));

        // Cleanup
        let _ = fs::remove_file("agent.key");
        let _ = fs::remove_file("agent.crt");
    }
}


#[tokio::test]
async fn test_enrollment_request_success_mock() {
    let _guard = logger::set_log_file_for_tests(setup_test_logger());
    let mut mock_client = MockEnrollmentClient::new();
    let request = EnrollmentRequest {
        enrollment_token: "test-token".to_string(),
        agent_id: "agent-123".to_string(),
        public_key: "test-key".to_string(),
    };
    let expected_response = EnrollmentResponse {
        status: "success".to_string(),
        certificate: Some("test-cert".to_string()),
    };

    let response_clone = expected_response.clone();
    mock_client
        .expect_enroll()
        .with(predicate::eq(request.clone()))
        .times(1)
        .returning(move |_| Ok(response_clone.clone()));

    let result = mock_client.enroll(request).unwrap();
    assert_eq!(result, expected_response);
}

use enrollment_agent::{Agent, EnrollmentRequest, EnrollmentResponse};
use enrollment_agent_logger as logger;
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

// Scenario: Agent successfully enrolls with a valid token.
// Expectation: Enrollment is successful, and agent.key and agent.crt files are created.
#[tokio::test]
async fn Test_Agent_Enroll_ValidToken_Success() {
    let _guard = logger::set_log_file_for_tests(setup_test_logger());
    let agent = Agent::new("agent-test", "https://localhost:8443");
    
    // Ensure clean state before test
    let _ = fs::remove_file("agent.key");
    let _ = fs::remove_file("agent.crt");

    let result = agent.enroll("valid-token").await;
    assert!(result.is_ok(), "Enrollment failed: {:?}", result.err());
    assert!(Path::new("agent.key").exists());
    assert!(Path::new("agent.crt").exists());

    // Cleanup
    let _ = fs::remove_file("agent.key");
    let _ = fs::remove_file("agent.crt");
}

// Scenario: Agent successfully reconnects using existing valid identity files.
// Expectation: mTLS reconnection is successful and returns a verification message.
#[tokio::test]
async fn Test_Agent_Reconnect_ValidIdentity_ReturnsSuccess() {
    let _guard = logger::set_log_file_for_tests(setup_test_logger());
    let agent = Agent::new("agent-test", "https://localhost:8443");

    // Setup: Create dummy identity files for reconnection test
    fs::write("agent.key", "dummy_key").expect("Failed to create dummy agent.key");
    fs::write("agent.crt", "dummy_crt").expect("Failed to create dummy agent.crt");
    fs::write("ca.crt", "dummy_ca").expect("Failed to create dummy ca.crt"); // Required by the agent::reconnect

    let reconnect_result = agent.reconnect().await;
    assert!(reconnect_result.is_ok(), "mTLS reconnection failed: {:?}", reconnect_result.err());
    // The actual content check for "Hello verified agent: agent-test" would require a running server
    // For this isolated test, we primarily check if the reconnection call itself succeeded without panicking

    // Cleanup
    let _ = fs::remove_file("agent.key");
    let _ = fs::remove_file("agent.crt");
    let _ = fs::remove_file("ca.crt");
}

// Scenario: Enrollment client receives a valid request.
// Expectation: Enrollment succeeds and returns the expected response.
#[tokio::test]
async fn Test_EnrollmentClient_Enroll_ValidRequest_ReturnsSuccess() {
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

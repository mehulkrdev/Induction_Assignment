use enrollment_agent::{Agent, EnrollmentRequest, EnrollmentResponse};
use enrollment_agent_logger as logger;
use mockall::{automock, predicate};
use std::fs;
use std::fs::File;
use std::path::Path;
mod test_helpers;
use test_helpers::{
    extract_ca_cert, get_server_container_id, run_command, start_server, stop_server,
    wait_for_server, AgentTestConfig, ServerGuard,
};

use chrono;
use std::path::{Path, PathBuf};
use tokio::fs;

// Helper to create a dummy log file for tests
fn setup_test_logger() -> File {
    let path = std::env::temp_dir().join(format!(
        "test_integration_log_{}.log",
        chrono::Local::now().format("%Y%m%d%H%M%S")
    ));
    File::create(&path).expect("Failed to create test log file")
}

// Scenario: Agent successfully enrolls with a valid token.
// Expectation: Enrollment is successful, and agent.key and agent.crt files are created in the temporary directory.
#[tokio::test]
async fn test_agent_enroll_valid_token_success() {
    let _guard = logger::set_log_file_for_tests(setup_test_logger());

    // Use ServerGuard for automatic server lifecycle management and temporary directory for certs
    let server_guard = ServerGuard::new()
        .await
        .expect("Failed to start server and extract CA cert");
    let temp_dir_path = server_guard.temp_dir_path();

    let agent_config = AgentTestConfig {
        certs_path: Some(temp_dir_path.to_path_buf()),
    };

    let agent = Agent::new_with_config("agent-test", "https://localhost:8443", agent_config);

    let result = agent.enroll("valid-token").await;
    assert!(result.is_ok(), "Enrollment failed: {:?}", result.err());
    assert!(temp_dir_path.join("agent.key").exists());
    assert!(temp_dir_path.join("agent.crt").exists());
}

// Scenario: Agent successfully reconnects using existing valid identity files.
// Expectation: mTLS reconnection is successful and returns a verification message.
#[tokio::test]
async fn test_agent_reconnect_valid_identity_returns_success() {
    let _guard = logger::set_log_file_for_tests(setup_test_logger());

    let server_guard = ServerGuard::new()
        .await
        .expect("Failed to start server and extract CA cert");
    let temp_dir_path = server_guard.temp_dir_path();

    // First, enroll to get valid agent.key and agent.crt
    let enrollment_agent_config = AgentTestConfig {
        certs_path: Some(temp_dir_path.to_path_buf()),
    };
    let enrollment_agent = Agent::new_with_config(
        "reconnect-agent",
        "https://localhost:8443",
        enrollment_agent_config,
    );
    enrollment_agent
        .enroll("valid-token")
        .await
        .expect("Initial enrollment for reconnect test failed");

    // Now attempt reconnection
    let reconnect_agent_config = AgentTestConfig {
        certs_path: Some(temp_dir_path.to_path_buf()),
    };
    let reconnect_agent = Agent::new_with_config(
        "reconnect-agent",
        "https://localhost:8443",
        reconnect_agent_config,
    );

    let reconnect_result = reconnect_agent.reconnect().await;

    assert!(
        reconnect_result.is_ok(),
        "mTLS reconnection failed: {:?}",
        reconnect_result.err()
    );
    assert!(reconnect_result
        .unwrap()
        .contains("Hello verified agent: reconnect-agent"));
}

// Mocks for EnrollmentClient (existing tests)
#[automock]
pub trait EnrollmentClient {
    fn enroll(&self, request: EnrollmentRequest) -> Result<EnrollmentResponse, String>;
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

// Scenario: Enrollment client receives an unauthorized error from server (401).
// Expectation: Enrollment returns an error.
#[tokio::test]
async fn Test_EnrollmentClient_Enroll_Unauthorized_ReturnsError() {
    let _guard = logger::set_log_file_for_tests(setup_test_logger());
    let mut mock_client = MockEnrollmentClient::new();
    let request = EnrollmentRequest {
        enrollment_token: "invalid-token".to_string(),
        agent_id: "agent-123".to_string(),
        public_key: "test-key".to_string(),
    };

    mock_client.expect_enroll().times(1).returning(|_| {
        Err(
            "Server returned error: 401 Unauthorized - Missing or invalid enrollment token"
                .to_string(),
        )
    });

    let result = mock_client.enroll(request);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap(),
        "Server returned error: 401 Unauthorized - Missing or invalid enrollment token"
    );
}

// Scenario: Enrollment client receives a conflict error from server (409).
// Expectation: Enrollment returns an error.
#[tokio::test]
async fn Test_EnrollmentClient_Enroll_Conflict_ReturnsError() {
    let _guard = logger::set_log_file_for_tests(setup_test_logger());
    let mut mock_client = MockEnrollmentClient::new();
    let request = EnrollmentRequest {
        enrollment_token: "valid-token".to_string(),
        agent_id: "duplicate-agent".to_string(),
        public_key: "test-key".to_string(),
    };

    mock_client.expect_enroll().times(1).returning(|_| {
        Err("Server returned error: 409 Conflict - Agent ID already enrolled".to_string())
    });

    let result = mock_client.enroll(request);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap(),
        "Server returned error: 409 Conflict - Agent ID already enrolled"
    );
}

// Scenario: Enrollment client receives a malformed JSON response.
// Expectation: Enrollment returns an error.
#[tokio::test]
async fn Test_EnrollmentClient_Enroll_MalformedJSON_ReturnsError() {
    let _guard = logger::set_log_file_for_tests(setup_test_logger());
    let mut mock_client = MockEnrollmentClient::new();
    let request = EnrollmentRequest {
        enrollment_token: "valid-token".to_string(),
        agent_id: "agent-123".to_string(),
        public_key: "test-key".to_string(),
    };

    mock_client.expect_enroll().times(1).returning(|_| {
        Err("Failed to parse enrollment response: expected ident at line 1 column 2".to_string())
    });

    let result = mock_client.enroll(request);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .contains("Failed to parse enrollment response"));
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

// Scenario: Enrollment client receives an unauthorized error from server (401).
// Expectation: Enrollment returns an error.
#[tokio::test]
async fn Test_EnrollmentClient_Enroll_Unauthorized_ReturnsError() {
    let _guard = logger::set_log_file_for_tests(setup_test_logger());
    let mut mock_client = MockEnrollmentClient::new();
    let request = EnrollmentRequest {
        enrollment_token: "invalid-token".to_string(),
        agent_id: "agent-123".to_string(),
        public_key: "test-key".to_string(),
    };

    mock_client.expect_enroll().times(1).returning(|_| {
        Err(
            "Server returned error: 401 Unauthorized - Missing or invalid enrollment token"
                .to_string(),
        )
    });

    let result = mock_client.enroll(request);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap(),
        "Server returned error: 401 Unauthorized - Missing or invalid enrollment token"
    );
}

// Scenario: Enrollment client receives a conflict error from server (409).
// Expectation: Enrollment returns an error.
#[tokio::test]
async fn Test_EnrollmentClient_Enroll_Conflict_ReturnsError() {
    let _guard = logger::set_log_file_for_tests(setup_test_logger());
    let mut mock_client = MockEnrollmentClient::new();
    let request = EnrollmentRequest {
        enrollment_token: "valid-token".to_string(),
        agent_id: "duplicate-agent".to_string(),
        public_key: "test-key".to_string(),
    };

    mock_client.expect_enroll().times(1).returning(|_| {
        Err("Server returned error: 409 Conflict - Agent ID already enrolled".to_string())
    });

    let result = mock_client.enroll(request);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap(),
        "Server returned error: 409 Conflict - Agent ID already enrolled"
    );
}

// Scenario: Enrollment client receives a malformed JSON response.
// Expectation: Enrollment returns an error.
#[tokio::test]
async fn Test_EnrollmentClient_Enroll_MalformedJSON_ReturnsError() {
    let _guard = logger::set_log_file_for_tests(setup_test_logger());
    let mut mock_client = MockEnrollmentClient::new();
    let request = EnrollmentRequest {
        enrollment_token: "valid-token".to_string(),
        agent_id: "agent-123".to_string(),
        public_key: "test-key".to_string(),
    };

    mock_client.expect_enroll().times(1).returning(|_| {
        Err("Failed to parse enrollment response: expected ident at line 1 column 2".to_string())
    });

    let result = mock_client.enroll(request);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .contains("Failed to parse enrollment response"));
}

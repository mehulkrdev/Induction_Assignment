#[cfg(test)] //compile this module only during testing
mod tests {
    use crate::*;
    use mockall::automock; // only import this during testing.
    use crate::logger;
    use std::fs::File;

    // Helper to create a dummy log file for tests
    fn setup_test_logger() -> File {
        let path = std::env::temp_dir().join(format!("test_enrollment_log_{}.log", chrono::Local::now().format("%Y%m%d%H%M%S")));
        File::create(&path).expect("Failed to create test log file")
    }

    #[tokio::test] //tokio is rust async runtime (Run this test inside Tokio\"s runtime)
    async fn test_server_discovery() {
        let _guard = logger::set_log_file_for_tests(setup_test_logger());
        log_entry!("Starting test_server_discovery");

        let mut mock_discovery = MockDiscovery::new();
        mock_discovery
            .expect_discover_server()
            .times(1)
            .returning(|| Ok("localhost:8443".to_string()));

        let addr = mock_discovery.discover_server().unwrap(); // unwrap() means I expect this to succeed. Crash if it doesn\"t. (discover_server.value())
        log_entry!("Discovered server address: {}", addr);
        assert_eq!(addr, "localhost:8443");
    }

    #[tokio::test]
    async fn test_enrollment_request_success() {
        let mut mock_client = MockEnrollmentClient::new();
        let request = EnrollmentRequest {
            enrollment_token: "test-token".to_string(),
            agent_id: "agent-123".to_string(),
            public_key: "test-key".to_string(),
        };
        let expected_response = EnrollmentResponse {
            status: "success".to_string(),
            certificate: Some("test-cert".to_string()), // std::optional<std::string>{\"test-cert\"}
        };

        let response_clone = expected_response.clone();
        mock_client
            .expect_enroll()// EXPECT_CALL(mockClient, enroll(...))
            .with(mockall::predicate::eq(request.clone())) // .with(mockall::predicate::eq(request.clone()))
            .times(1)
            .returning(move |_| Ok(response_clone.clone())); // WillOnce(Return(successResponse));

        let result = mock_client.enroll(request).await.unwrap(); //auto result = co_await client.enroll(request);
        log_entry!("Enrollment successful: {:?}", result);
        assert_eq!(result, expected_response);
    }

    #[tokio::test]
    async fn test_enrollment_request_failure() {
        let _guard = logger::set_log_file_for_tests(setup_test_logger());
        log_entry!("Starting test_enrollment_request_failure");

        let mut mock_client = MockEnrollmentClient::new();
        let request = EnrollmentRequest {
            enrollment_token: "test-token".to_string(),
            agent_id: "agent-123".to_string(),
            public_key: "test-key".to_string(),
        };

        mock_client
            .expect_enroll()
            .times(1)
            .returning(|_| Err("Unauthorized".to_string()));

        let result = mock_client.enroll(request).await;
        assert!(result.is_err());
        log_entry!("Enrollment failed: {:?}", result);
        assert_eq!(result.unwrap_err(), "Unauthorized");
    }
}



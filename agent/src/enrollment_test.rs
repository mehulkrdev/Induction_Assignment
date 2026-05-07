#[cfg(test)]
mod tests {
    use crate::*;

    #[tokio::test]
    async fn test_server_discovery() {
        let mut mock_discovery = MockDiscovery::new();
        mock_discovery
            .expect_discover_server()
            .times(1)
            .returning(|| Ok("localhost:8443".to_string()));

        let addr = mock_discovery.discover_server().unwrap();
        assert_eq!(addr, "localhost:8443");
    }

    #[tokio::test]
    async fn test_enrollment_request_success() {
        let mut mock_client = MockEnrollmentClient::new();
        let request = EnrollmentRequest {
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
            .with(mockall::predicate::eq(request.clone()))
            .times(1)
            .returning(move |_| Ok(response_clone.clone()));

        let result = mock_client.enroll(request).await.unwrap();
        assert_eq!(result, expected_response);
    }

    #[tokio::test]
    async fn test_enrollment_request_failure() {
        let mut mock_client = MockEnrollmentClient::new();
        let request = EnrollmentRequest {
            agent_id: "agent-123".to_string(),
            public_key: "test-key".to_string(),
        };

        mock_client
            .expect_enroll()
            .times(1)
            .returning(|_| Err("Unauthorized".to_string()));

        let result = mock_client.enroll(request).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Unauthorized");
    }
}

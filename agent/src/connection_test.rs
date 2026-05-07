
#[cfg(test)]
mod tests {
    use tokio::net::TcpStream;

    #[tokio::test]
    async fn test_agent_connection_to_server_port_8443() {
        let addr = "127.0.0.1:8443";
        let stream = TcpStream::connect(addr).await;
        assert!(stream.is_ok(), "Agent failed to connect to the server on port 8443");
    }
}

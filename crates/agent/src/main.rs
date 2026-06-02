use enrollment_agent::Agent;
use std::env;
use enrollment_agent_logger;
use url::Url;

const MAX_URL_LENGTH: usize = 256;
const MAX_TOKEN_LENGTH: usize = 64;
const MAX_AGENT_ID_LENGTH: usize = 32;

fn validate_server_url(url: &str) -> Result<(), String> {
    if url.len() > MAX_URL_LENGTH {
        return Err(format!("Server URL exceeds maximum length of {} characters", MAX_URL_LENGTH));
    }
    if !url.starts_with("https://") {
        return Err("Server URL must start with 'https://'".to_string());
    }
    match Url::parse(url) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Invalid server URL format: {}", e)),
    }
}

fn validate_input_length(input: &str, max_len: usize, input_name: &str) -> Result<(), String> {
    if input.len() > max_len {
        return Err(format!("{} exceeds maximum length of {} characters", input_name, max_len));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_server_url_valid() {
        assert!(validate_server_url("https://localhost:8443").is_ok());
        assert!(validate_server_url("https://example.com/some/path").is_ok());
    }

    #[test]
    fn test_validate_server_url_invalid_scheme() {
        assert!(validate_server_url("http://localhost:8443").is_err());
        assert!(validate_server_url("ftp://localhost:8443").is_err());
        assert!(validate_server_url("localhost:8443").is_err());
    }

    #[test]
    fn test_validate_server_url_too_long() {
        let long_url = format!("https://example.com/{}", "a".repeat(250)); // Total length > 256
        assert!(validate_server_url(&long_url).is_err());
    }

    #[test]
    fn test_validate_server_url_invalid_format() {
        assert!(validate_server_url("https://invalid-url!@#").is_err());
        assert!(validate_server_url("https://").is_err());
    }

    #[test]
    fn test_validate_input_length_valid() {
        assert!(validate_input_length("short_token", MAX_TOKEN_LENGTH, "Token").is_ok());
        assert!(validate_input_length("agent123", MAX_AGENT_ID_LENGTH, "Agent ID").is_ok());
    }

    #[test]
    fn test_validate_input_length_too_long() {
        let long_token = "a".repeat(MAX_TOKEN_LENGTH + 1);
        assert!(validate_input_length(&long_token, MAX_TOKEN_LENGTH, "Token").is_err());

        let long_agent_id = "b".repeat(MAX_AGENT_ID_LENGTH + 1);
        assert!(validate_input_length(&long_agent_id, MAX_AGENT_ID_LENGTH, "Agent ID").is_err());
    }
}



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 5 {
        eprintln!("Usage: agent <command> <token> <agent_id> <server_url>");
        std::process::exit(1);
    }

    let command = &args[1];
    let token = &args[2];
    let agent_id = &args[3];
    let server_url = &args[4];

    if let Err(e) = validate_input_length(token, MAX_TOKEN_LENGTH, "Token") {
        eprintln!("Validation error: {}", e);
        std::process::exit(1);
    }
    if let Err(e) = validate_input_length(agent_id, MAX_AGENT_ID_LENGTH, "Agent ID") {
        eprintln!("Validation error: {}", e);
        std::process::exit(1);
    }
    if let Err(e) = validate_server_url(server_url) {
        eprintln!("Validation error: {}", e);
        std::process::exit(1);
    }

    enrollment_agent_logger::init_logger("agent")
        .expect("Failed to initialize agent logger");
    let agent = Agent::new(agent_id, server_url);

    match command.as_str() {
        "enroll" => {
            if let Err(e) = agent.enroll(token).await {
                eprintln!("Enrollment failed: {}", e);
                std::process::exit(1);
            }
            println!("Enrollment successful");
        }
        "reconnect" => {
            match agent.reconnect().await {
                Ok(response) => println!("Reconnection response: {}", response),
                Err(e) => {
                    eprintln!("Reconnection failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        }
    }

    Ok(())
}

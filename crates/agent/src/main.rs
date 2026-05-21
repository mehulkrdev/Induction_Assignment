use enrollment_agent::Agent;
use std::env;
use enrollment_agent_logger;

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

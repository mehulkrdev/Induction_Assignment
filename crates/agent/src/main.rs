use enrollment_agent::Agent;
use std::env;

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

    let agent = Agent::new(agent_id, server_url);

    match command.as_str() {
        "enroll" => {
            agent.enroll(token).await?;
            println!("Enrollment successful");
        }
        "reconnect" => {
            let response = agent.reconnect().await?;
            println!("Reconnection response: {}", response);
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        }
    }

    Ok(())
}

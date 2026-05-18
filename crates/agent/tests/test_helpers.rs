use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

// Helper to run shell commands and capture output
pub async fn run_command(command: &str, args: &[&str]) -> Result<String, String> {
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("wsl");
        c.arg(command);
        c
    } else {
        Command::new(command)
    };

    let output = cmd.args(args).output().map_err(|e| {
        format!(
            "Failed to execute command \'{}\' with args {:?}: {}",
            command, args, e
        )
    })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "Command \'{}\' with args {:?} failed with status {}. Stderr: {}\nStdout: {}",
            command,
            args,
            output.status,
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        ))
    }
}

// Starts the Docker Compose server
pub async fn start_server() -> Result<(), String> {
    println!("Starting Docker Compose server...");
    run_command("docker", &["compose", "up", "-d", "server"]).await?;
    println!("Docker Compose server started.");
    Ok(())
}

// Waits for the server to be ready on a given port
pub async fn wait_for_server(port: u16) -> Result<(), String> {
    let addr = format!("127.0.0.1:{}", port);
    println!("Waiting for server to be ready on {}...", addr);
    let mut attempts = 0;
    let max_attempts = 30; // 30 seconds timeout with 1-second initial delay
    let delay = Duration::from_secs(1);

    while attempts < max_attempts {
        match timeout(Duration::from_secs(1), TcpStream::connect(&addr)).await {
            Ok(Ok(_)) => {
                println!("Server on {} is ready.", addr);
                return Ok(());
            }
            Ok(Err(e)) => {
                // Connection refused is expected while server is starting
                if e.kind() == io::ErrorKind::ConnectionRefused
                    || e.kind() == io::ErrorKind::BrokenPipe
                {
                    // println!("Attempt {}/{}: Connection refused. Retrying...", attempts + 1, max_attempts);
                } else {
                    return Err(format!("Error connecting to server on {}: {}", addr, e));
                }
            }
            Err(_) => {
                // Timeout
                println!(
                    "Attempt {}/{}: Connection timed out. Retrying...",
                    attempts + 1,
                    max_attempts
                );
            }
        }
        attempts += 1;
        tokio::time::sleep(delay).await;
        // No exponential backoff for now, simple fixed delay, can be improved.
    }
    Err(format!(
        "Server on {} did not become ready after {} attempts.",
        addr, max_attempts
    ))
}

// Stops the Docker Compose server
pub async fn stop_server() -> Result<(), String> {
    println!("Stopping Docker Compose server...");
    run_command("docker", &["compose", "down"]).await?;
    println!("Docker Compose server stopped.");
    Ok(())
}

// Gets the Docker Compose server container ID
pub async fn get_server_container_id() -> Result<String, String> {
    println!("Getting server container ID...");
    let container_id = run_command("docker", &["compose", "ps", "-q", "server"]).await?;
    if container_id.is_empty() {
        Err("Could not get server container ID. Is the server running?".to_string())
    } else {
        println!("Server container ID: {}", container_id);
        Ok(container_id)
    }
}

// Extracts the CA certificate from the running server container to a temporary directory
pub async fn extract_ca_cert(temp_dir: &Path, container_id: &str) -> Result<PathBuf, String> {
    let dest_path = temp_dir.join("ca.crt");
    let container_src = format!("{}:/app/data/ca.crt", container_id);
    
    // On Windows, temp_dir.path() returns a Windows path (e.g. C:\Users\...).
    // If we call 'wsl docker cp', WSL docker expects a Linux-style path (e.g. /mnt/c/Users/...).
    let wsl_dest_path = if cfg!(windows) {
        let path_str = dest_path.to_string_lossy().to_string();
        // Simple conversion for common C:\ paths to /mnt/c/
        path_str.replace("\\", "/").replace("C:", "/mnt/c").replace("c:", "/mnt/c")
    } else {
        dest_path.to_string_lossy().to_string()
    };

    println!(
        "Extracting CA certificate from {} to {} (WSL path: {})...",
        container_src,
        dest_path.display(),
        wsl_dest_path
    );

    run_command(
        "docker",
        &["cp", &container_src, &wsl_dest_path],
    )
    .await?;
    println!("CA certificate extracted to {}.", dest_path.display());
    Ok(dest_path)
}

// RAII guard to ensure server cleanup
pub struct ServerGuard {
    _temp_dir: tempfile::TempDir,
    is_cleaned_up: bool,
}

impl ServerGuard {
    pub async fn new() -> Result<Self, String> {
        // Ensure the server is stopped before starting a new one (cleanup from previous failed run)
        let _ = stop_server().await;

        start_server().await?;
        wait_for_server(8443).await?;
        wait_for_server(8444).await?;

        let temp_dir = tempfile::tempdir()
            .map_err(|e| format!("Failed to create temporary directory: {}", e))?;
        let container_id = get_server_container_id().await?;
        extract_ca_cert(temp_dir.path(), &container_id).await?;

        Ok(Self {
            _temp_dir: temp_dir,
            is_cleaned_up: false,
        })
    }

    pub async fn cleanup(&mut self) -> Result<(), String> {
        if !self.is_cleaned_up {
            println!("Stopping Docker Compose server via explicit cleanup...");
            let result = stop_server().await;
            if let Err(e) = &result {
                eprintln!("Error stopping server during cleanup: {}", e);
            }
            self.is_cleaned_up = result.is_ok();
            result
        } else {
            Ok(())
        }
    }

    pub fn ca_cert_path(&self) -> PathBuf {
        self._temp_dir.path().join("ca.crt")
    }

    pub fn temp_dir_path(&self) -> &Path {
        self._temp_dir.path()
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        if !self.is_cleaned_up {
            eprintln!("WARNING: ServerGuard was dropped without explicit cleanup. Docker containers might still be running.");
        }
    }
}

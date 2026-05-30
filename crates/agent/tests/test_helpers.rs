use std::path::Path;
use regex::Regex;
use std::path::PathBuf;
use tokio::process::Command;
use tokio::time::{Duration};
use sha2::{Digest, Sha256};
use hex;

// Helper to run shell commands and capture output
pub async fn run_command(command: &str, args: &[&str]) -> Result<String, String> {
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("wsl");
        c.arg(command);
        c
    } else {
        Command::new(command)
    };

    let output = cmd.args(args).output().await.map_err(|e| {
        format!(
            "Failed to execute command \'{}\'' with args {:?}: {}",
            command, args, e
        )
    })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "Command \'{}\'' with args {:?} failed with status {}. Stderr: {}\nStdout: {}",
            command,
            args,
            output.status,
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        ))
    }
}

// Starts the Docker Compose server if not already running
pub async fn start_server() -> Result<(), String> {
    println!("Ensuring Docker Compose server is running...");
    // We do NOT clean ./data here to avoid destroying shared state during parallel tests
    // or disrupting the server. Destructive cleanup is removed.
    run_command("docker", &["compose", "up", "-d", "server"]).await?;
    println!("Docker Compose server 'up' command executed.");
    Ok(())
}

// Diagnostic helper to dump container status and logs
pub async fn print_diagnostics() {
    println!("--- STARTUP DIAGNOSTICS ---");
    let _ = run_command("docker", &["compose", "ps"]).await.map(|out| println!("PS:\n{}", out));
    let _ = run_command("docker", &["compose", "logs", "--tail", "50", "server"]).await.map(|out| println!("LOGS:\n{}", out));
    println!("--- END DIAGNOSTICS ---");
}

// Waits for the server to be ready on a given port using HTTPS application-layer check
pub async fn wait_for_server_https(port: u16) -> Result<(), String> {
    let url = format!("https://localhost:{}", port);
    println!("Waiting for HTTPS readiness on {}...", url);
    
    let mut attempts = 0;
    let max_attempts = 45; // Increased timeout for slow environments
    let delay = Duration::from_secs(1);

    while attempts < max_attempts {
        // Use curl -k to check if the server responds at the HTTPS layer
        // -s: silent, -f: fail on 4xx/5xx, -k: insecure (ignore cert validation for health check)
        // -I: fetch headers only (optimized) or just check exit code
        let result = run_command("curl", &["-skv", "-I", &url]).await;
        
        // On Windows with WSL, curl might return success even if headers show 400, 
        // OR it might return error. We check both.
        let output = match &result {
            Ok(out) => out.clone(),
            Err(err) => err.clone(),
        };

        // If we see TLS handshake activity or any HTTP response, the server is up.
        if output.contains("HTTP/") || output.contains("TLS handshake") || output.contains("Connected to") {
             println!("Server on {} responded (application layer up).", url);
             return Ok(());
        }

        if result.is_ok() {
            println!("Server on {} is ready (HTTPS).", url);
            return Ok(());
        }

        attempts += 1;
        tokio::time::sleep(delay).await;
    }

    print_diagnostics().await;
    Err(format!(
        "Server on {} did not become HTTPS-ready after {} attempts.",
        url, max_attempts
    ))
}

// Stops the Docker Compose server - usually called only at the end of all tests
pub async fn stop_server() -> Result<(), String> {
    println!("Stopping Docker Compose server...");
    run_command("docker", &["compose", "down"]).await?;
    println!("Docker Compose server stopped.");
    Ok(())
}

// Gets the Docker Compose server container ID
pub async fn get_server_container_id() -> Result<String, String> {
    let container_id = run_command("docker", &["compose", "ps", "-q", "server"]).await?;
    if container_id.is_empty() {
        Err("Could not get server container ID. Is the server running?".to_string())
    } else {
        Ok(container_id)
    }
}

// Extracts the CA certificate from the running server container to a temporary directory
pub async fn extract_ca_cert(temp_dir: &Path, container_id: &str) -> Result<PathBuf, String> {
    let dest_path = temp_dir.join("ca.crt");
    let container_src = format!("{}:/app/data/ca.crt", container_id);
    
    let wsl_dest_path = if cfg!(windows) {
        let path_str = dest_path.to_string_lossy().to_string();
        let re = Regex::new(r"^([A-Za-z]):").unwrap();
        re.replace_all(&path_str.replace("\\", "/"), |caps: &regex::Captures| {
            format!("/mnt/{}", caps[1].to_lowercase())
        }).to_string()
    } else {
        dest_path.to_string_lossy().to_string()
    };

    println!("Extracting CA certificate...");
    run_command("docker", &["cp", &container_src, &wsl_dest_path]).await?;
    Ok(dest_path)
}

// RAII guard to ensure server cleanup
pub struct ServerGuard {
    _temp_dir: tempfile::TempDir,
    ca_cert_path: PathBuf,
    ca_cert_fingerprint: Option<String>,
}

impl ServerGuard {
    pub async fn new() -> Result<Self, String> {
        // Shared infrastructure: we don't 'down' anymore.
        start_server().await?;
        
        // Robust HTTPS polling for both services
        wait_for_server_https(8443).await?;
        wait_for_server_https(8444).await?;

        let temp_dir = tempfile::tempdir()
            .map_err(|e| format!("Failed to create temporary directory: {}", e))?;
        let container_id = get_server_container_id().await?;
        let ca_cert_path = extract_ca_cert(temp_dir.path(), &container_id).await?;

        let ca_cert_pem = tokio::fs::read(&ca_cert_path).await
            .map_err(|e| format!("Failed to read CA cert for fingerprint calculation: {}", e))?;
        let ca_cert_fingerprint = calculate_sha256(&ca_cert_pem);

        Ok(Self {
            _temp_dir: temp_dir,
            ca_cert_path,
            ca_cert_fingerprint: Some(ca_cert_fingerprint),
        })
    }

    pub async fn cleanup(&mut self) -> Result<(), String> {
        // We no longer stop the server per test. 
        // Shared infrastructure stays up until the test process ends or is manually stopped.
        Ok(())
    }

    pub fn ca_cert_path(&self) -> PathBuf {
        self.ca_cert_path.clone()
    }

    pub fn temp_dir_path(&self) -> &Path {
        self._temp_dir.path()
    }

    pub fn ca_cert_fingerprint(&self) -> Option<String> {
        self.ca_cert_fingerprint.clone()
    }
}

fn calculate_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

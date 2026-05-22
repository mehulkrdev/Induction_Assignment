# Secure Enrollment & mTLS Communication Platform

## Description

This project implements a secure cross-platform enrollment and mutual TLS (mTLS) communication system using:

*   A Rust-based Windows Agent
*   A Go-based Debian Server running inside Docker on WSL

The platform enables secure device enrollment using certificate-based authentication. The agent dynamically generates its own cryptographic identity, enrolls securely with the server over HTTPS, receives a signed client certificate, and later establishes a fully authenticated mTLS connection.

The implementation follows practical Test-Driven Development (TDD) principles, clean repository structuring, Dockerized deployment, and modern TLS security practices.

---

# Problem Statement

The goal of this assignment was to design and implement a secure enrollment mechanism between:

*   A Rust-based client agent running on Windows
*   A Go-based backend server running on Debian (inside WSL/Docker)

The system needed to:

*   Establish secure communication
*   Generate and manage cryptographic identities
*   Perform certificate-based enrollment
*   Support mutual TLS authentication
*   Follow TDD-oriented development practices
*   Use Docker for isolated deployment
*   Maintain proper project organization and testing structure

---

# Project Execution Model

This project operates on a split architecture leveraging the strengths of Windows and WSL/Linux:

*   **Go Server (Containerized)**: The Go-based enrollment and mTLS server runs within a Docker container, hosted by WSL2. This provides an isolated, consistent Linux environment for the server and its Certificate Authority (CA). The server dynamically generates its CA and TLS certificates at runtime.
*   **Rust Agent (Native in WSL)**: The Rust-based agent and its associated tests are designed to run natively within a WSL/Linux terminal. This allows direct interaction with the Dockerized server via `localhost` and simplifies tooling by consolidating all development into a Linux environment.
*   **Dynamic TLS Certificates**: The server generates a self-signed root CA and server certificates on startup. Client agents enroll by submitting public keys, which the server signs to issue client certificates. The `ca.crt` must be extracted from the running Docker container to establish trust with the server.

---

# Architecture Overview

## Components

### Rust Agent

Responsible for:

*   Generating ECDSA P-256 keypairs
*   Sending enrollment requests
*   Persisting issued certificates
*   Establishing mTLS communication

### Go Server

Responsible for:

*   Generating a local Certificate Authority (CA)
*   Generating server certificates
*   Signing agent certificates
*   Hosting HTTPS and mTLS endpoints

### Docker + WSL

Responsible for:

*   Debian-based isolated deployment
*   Consistent runtime environment
*   Port exposure and networking

---

# Features Implemented

## Secure Enrollment

*   HTTPS-based enrollment endpoint (`8443`)
*   Enrollment token validation
*   Public key submission from agent
*   Server-side certificate signing

## Mutual TLS (mTLS)

*   Client certificate authentication
*   CA-based trust validation
*   Bidirectional identity verification
*   Secure communication over port `8444`

## Certificate Infrastructure

*   Self-signed local Certificate Authority
*   Dynamic server certificate generation
*   Dynamic client certificate generation
*   PEM encoding/decoding support

## Persistent Identity

The agent stores:

*   Private key (`agent.key`)
*   Public key (`agent.pub`)
*   Signed certificate (`agent.crt`)

## Dockerized Deployment

*   Go server runs inside Docker
*   Debian-based runtime environment
*   Exposed TLS ports for enrollment and mTLS

## TDD-Oriented Structure

*   Integration tests separated from implementation
*   Dedicated `tests/` directories
*   Mock-based behavioral testing

---

# Technologies Used

## Rust

*   tokio
*   reqwest
*   rustls
*   p256
*   pkcs8
*   serde

## Go

*   crypto/tls
*   crypto/x509
*   net/http
*   encoding/pem

## DevOps / Environment

*   Docker
*   Docker Compose
*   WSL (Windows Subsystem for Linux)

---

# Project Directory Structure

```text
Assignment/
│
├── crates/
│   ├── agent/
│   │   ├── src/
│   │   │   └── lib.rs
│   │   │
│   │   ├── tests/
│   │   │   └── enrollment_test.rs
│   │   │
│   │   └── Cargo.toml
│   │
│   └── agent_logger/
│       ├── src/
│       │   └── lib.rs
│       │
│       └── Cargo.toml
│
├── server/
│   ├── main.go
│   ├── Dockerfile
│   │
│   ├── pkg/
│   │   ├── certutil/
│   │   └── logger/
│   │
│   ├── tests/
│   │   └── enrollment_test.go
│   │
│   └── go.mod
│
├── memorybank/
│
├── docker-compose.yml
│
└── README.md
```

---

# Enrollment + mTLS Flow

## Step 1 — Server Bootstrap

The Go server:

*   Generates a local CA
*   Generates server certificates
*   Starts HTTPS server on `8443`
*   Starts mTLS server on `8444`

## Step 2 — Agent Key Generation

The Rust agent:

*   Generates ECDSA P-256 keypair
*   Stores:

    *   `agent.key`
    *   `agent.pub`

## Step 3 — Enrollment

The agent:

*   Sends public key to `/enroll`
*   Includes enrollment token
*   Receives signed certificate from server

## Step 4 — Certificate Persistence

The agent stores:

*   `agent.crt`

## Step 5 — mTLS Communication

The agent:

*   Presents certificate to server
*   Proves ownership of private key
*   Establishes mutual TLS connection

The server:

*   Verifies client certificate against CA
*   Authenticates agent identity
*   Grants access to secure endpoint

---

# How the Assignment Was Handled

The implementation was developed incrementally using a TDD-oriented workflow.

## Initial Phase

*   Designed project structure
*   Set up Rust and Go services
*   Created Dockerized Go environment

## Security Implementation

*   Implemented CA generation
*   Added HTTPS support
*   Added client certificate signing
*   Added mTLS endpoint

## Refactoring

*   Removed dead code
*   Removed unused discovery components
*   Moved tests into dedicated `tests/` directories
*   Improved certificate encapsulation

## Validation

The complete system was manually verified by:

*   Enrolling an agent
*   Receiving signed certificates
*   Establishing mTLS communication
*   Verifying certificate-based authentication

---

# Test Strategy (TDD)

## Rust Tests

Located in:

```text
agent/tests/
```
To run `enrollment_test.rs`, navigate to the `crates/agent` directory within your WSL terminal and then execute the following command:

```bash
cd crates/agent && cargo test --test enrollment_test -- --nocapture
```

Tests include:

*   Enrollment flow validation
*   Certificate persistence
*   mTLS reconnection behavior

## Go Tests

Located in:

```text
server/tests/
```

To run `enrollment_test.go`, which is a Go test for the server, you should use Docker Compose from the project root in your WSL terminal:

```bash
docker compose run --rm server go test -v ./...
```

This command starts a temporary container for the server service, runs all Go tests within it (including `enrollment_test.go`), and then removes the container (`--rm`). This ensures the tests run in the same isolated Linux environment as the server itself.

Tests include:

*   Enrollment endpoint validation
*   Token validation
*   Certificate signing logic

---

# Setup & Execution Guide (WSL-First Workflow)

This guide assumes you are working within a **WSL/Linux terminal** (e.g., Ubuntu, Debian). All commands are provided for this environment. If you prefer to execute from a Windows terminal (PowerShell/CMD), refer to the "Windows Host Notes" section below.

## Prerequisites

*   **WSL2**: Installed and configured (e.g., Ubuntu or Debian).
*   **Docker & Docker Compose**: Installed within WSL2 and running.
*   **Rustup**: Installed in WSL2 (for Rust toolchain).
*   **build-essential**: Installed in WSL2 for Rust linker dependencies (`sudo apt install build-essential`).
*   **OpenSSL**: Installed in WSL2 (for manual key/certificate operations: `sudo apt install openssl`).
*   **jq**: Installed in WSL2 (for parsing JSON responses in manual verification: `sudo apt install jq`).

## Step 1 — Navigate to Project Root

Open your WSL terminal and navigate to the cloned project directory. Assuming you cloned it to your Windows user directory (replace `<your-username>` and the path accordingly):

```bash
cd /mnt/c/Users/<your-username>/path/to/Assignment
# or, if you have a symlink or other setup:
# cd ~/Assignment
```

## Step 2 — Start the Go Server (Dockerized)

The server is dockerized and dynamically generates its own Certificate Authority (CA) and server certificates on startup. It exposes port `8443` for enrollment (HTTPS) and `8444` for mTLS communication.

1.  Build the server Docker image and start the server using Docker Compose in detached mode (`-d`):
    ```bash
    docker compose build server && docker compose up -d server --force-recreate
    ```
    *   `--force-recreate` ensures a fresh container is created, regenerating the CA and certificates.
2.  Verify the server is running:
    ```bash
    docker ps
    ```
    *You should see `assignment-server-1` running and exposing ports `8443` (Enrollment) and `8444` (mTLS).*

## Step 3 — Extract the CA Certificate

The Rust agent and manual `curl` commands require the server's CA certificate to establish trust. Since the CA is dynamically generated, you must extract it from the running container.

1.  Create a `data/` directory at the project root if it doesn't exist. This directory is typically `.gitignore`d for ephemeral runtime artifacts:
    ```bash
    mkdir -p data
    ```
2.  Copy the `ca.crt` from the running `assignment-server-1` container to your local `data/` directory:
    ```bash
    docker cp assignment-server-1:/app/data/ca.crt ./data/ca.crt
    ```
    *   **Important**: If you stop and restart the server with `--force-recreate`, a *new* CA will be generated. You **must** re-run this `docker cp` command to get the updated `ca.crt`.

## Step 4 — Run the Rust Agent (Automated Verification)

We use the Rust agent's built-in integration tests to perform an end-to-end verification of the entire flow: Enrollment -> Certificate Persistence -> mTLS Reconnection.

1.  Navigate to the agent's crate directory:
    ```bash
    cd crates/agent
    ```
2.  Run the integration tests. These tests automatically locate `ca.crt` from the project root's `data/` directory using workspace-root resolution logic:
    ```bash
    cargo test --test enrollment_test -- --nocapture
    ```

### How to identify automated verification success:

*   **Test Result**: Look for output like `running 6 tests` followed by `test ... ok` for all tests.
*   **Log Output**: The agent logs its progress. In the `--nocapture` output, look for:
    *   `Starting enrollment for agent: agent-test`
    *   `Enrollment successful. Certificate and key saved.`
    *   `Attempting mTLS reconnection for agent: agent-test`
    *   `mTLS reconnection successful: Hello verified agent: agent-test`
*   **Error Handling Tests**: Verify that tests for HTTP 401, 409, and malformed JSON responses pass, indicating robust error handling.

## Step 5 — Manual Verification (Using Curl)

This section allows you to manually verify the enrollment and mTLS flow using `curl` and `openssl`, bypassing the Rust agent. **Ensure the Go server is running and `ca.crt` has been extracted to `./data/ca.crt` (see Steps 2 & 3) before proceeding.**

Follow these steps from the **project root** (`Assignment/`):

### A. Generate Agent Keypair

Generate a new ECDSA P-256 keypair for the manual agent:

```bash
openssl ecparam -name prime256v1 -genkey -noout -out manual_agent.key
openssl ec -in manual_agent.key -pubout -out manual_agent.pub
```

### B. Prepare and Send Enrollment Request

To avoid complex shell escaping issues with the multiline public key, we use `jq` to build the JSON payload and pipe it directly to `curl`:

```bash
# Build JSON payload and enroll in one command
jq -n --arg agent_id "manual-agent" \
      --arg token "valid-token" \
      --arg pubkey "$(cat manual_agent.pub)" \
      '{agent_id: $agent_id, enrollment_token: $token, public_key: $pubkey}' | \
curl -sk https://localhost:8443/enroll \
     -H "Content-Type: application/json" \
     -d @- | jq -r '.certificate' > manual_agent.crt
```

*   **Verification**: Ensure `manual_agent.crt` exists and starts with `-----BEGIN CERTIFICATE-----`.
*   **Troubleshooting**: If `manual_agent.crt` is empty, ensure the Go server is running and port `8443` is accessible.

*   **Verification**: Ensure `manual_agent.crt` exists in your project root and contains a valid PEM-encoded certificate.
*   **Expected Error Case (Duplicate)**: If an agent with `manual-agent` ID is already enrolled, the server will return an HTTP 409 Conflict. The `curl` command might still execute but save an empty or malformed `manual_agent.crt`.

**Troubleshooting**: If enrollment fails with failed to sign certificate: invalid or empty PEM block containing public key, previously generated keys may be malformed, empty, stale, or in an unsupported format. Delete existing agent keys/certificates and regenerate a fresh EC keypair before retrying enrollment:
```bash
rm -f manual_agent.key manual_agent.pub manual_agent.crt

openssl ecparam -name prime256v1 -genkey -noout -out manual_agent.key

openssl ec -in manual_agent.key -pubout -out manual_agent.pub
```
**Verification** : Run cat manual_agent.pub and confirm the file contains a PEM block similar to:
```
-----BEGIN PUBLIC KEY-----
...
-----END PUBLIC KEY-----
```
Expected Error Case (Invalid Key Format): If the public key is in OpenSSH format (for example ssh-ed25519 ... or ecdsa-sha2-nistp256 ...) instead of PEM PKIX format, enrollment will fail and the Go server logs will contain:
failed to sign certificate: invalid or empty PEM block containing public key

### C. Verify mTLS Connection

Test the mutual TLS connection by accessing the secure mTLS endpoint on port `8444` using the generated client certificate and key, and the server's CA certificate.

```bash
curl -vk https://localhost:8444/secure \
  --cert manual_agent.crt \
  --key manual_agent.key \
  --cacert data/ca.crt
```

### How to identify manual verification success:

*   **HTTP Status**: Look for `HTTP/2 200` in the verbose output.
*   **Response Body**: The server should respond with `Hello verified agent: manual-agent`.
*   **TLS Handshake**: In the verbose output (`-v`), observe the mTLS handshake:
    *   Server requests client certificate (`Request CERT`).
    *   Client presents its certificate (`Certificate`).

## Step 6 — Cleanup (Optional)

To stop and remove the Docker containers, and clean up generated certificate files:

```bash
# Stop and remove Docker containers
docker compose down

# Remove generated certs and keys
rm -f data/ca.crt manual_agent.key manual_agent.pub manual_agent.crt
# If you ran the Rust agent, also remove:
# rm -f crates/agent/agent.key crates/agent/agent.pub crates/agent/agent.crt
```

---

# Troubleshooting

This section addresses common issues encountered during setup and execution.

*   **`cc` not found / Linker Issues (Rust)**:
    *   **Symptom**: `linker `cc` not found` or similar errors during `cargo build` or `cargo test`.
    *   **Cause**: Missing C/C++ build tools in your WSL environment.
    *   **Resolution**: Install `build-essential` in your WSL distribution: `sudo apt update && sudo apt install build-essential`.

*   **Nested Tokio Runtime Panic (Rust)**:
    *   **Symptom**: `panic: Cannot start a runtime from within a runtime.` or similar when running Rust tests.
    *   **Cause**: Occurs when an asynchronous test tries to create a new Tokio runtime while already inside another. This often happens in test cleanup logic (`drop` implementations).
    *   **Resolution**: Ensure `drop` implementations or any destructors do not contain asynchronous code that implicitly creates new runtimes. Refactor async cleanup to use `tokio::task::block_in_place` or ensure explicit runtime management outside of `drop`.

*   **Go Server Build Failures (`go.sum` or Imports)**:
    *   **Symptom**: Errors during `docker compose build server` related to `go.sum` mismatches or missing imports.
    *   **Cause**: Go module dependencies are out of sync or `go.sum` has been manually altered incorrectly.
    *   **Resolution**: Run `go mod tidy` in the `server/` directory from within WSL (you might need to temporarily enter the container or mount the directory) to synchronize dependencies and update `go.sum`.

*   **`Error: current directory is not a workspace root` (Rust)**:
    *   **Symptom**: When running `cargo test --test enrollment_test` from the project root (`Assignment/`), Rust fails to find the test.
    *   **Cause**: The `--test` flag expects to be run from the crate's directory (`crates/agent/`).
    *   **Resolution**: Always navigate into the `crates/agent/` directory before running `cargo test --test enrollment_test`. If running `cargo test --workspace`, you can remain at the project root.

*   **Connection Refused / Server Not Found**:
    *   **Symptom**: `curl: (7) Failed to connect to localhost port 8443: Connection refused`.
    *   **Cause**: Docker containers are not running or the server container failed to start correctly.
    *   **Resolution**: Verify Docker is running (`sudo service docker status` and `docker ps`). Check Docker Compose logs for errors (`docker compose logs server`). Ensure ports `8443` and `8444` are not in use by other applications.

*   **CA Certificate Mismatch / TLS Handshake Errors**:
    *   **Symptom**: `curl: (60) SSL certificate problem: self signed certificate in certificate chain` or similar TLS errors, especially after restarting the server.
    *   **Cause**: The server's CA certificate (`ca.crt`) has changed. This happens when the Docker container is recreated, as the CA is ephemeral.
    *   **Resolution**: You **must** re-run `docker cp assignment-server-1:/app/data/ca.crt ./data/ca.crt` to extract the *new* `ca.crt` after any server recreation. Ensure agents use this latest CA for trust.

*   **`jq` not found**:
    *   **Symptom**: `command not found: jq` during manual enrollment.
    *   **Cause**: `jq` is not installed in your WSL environment.
    *   **Resolution**: Install `jq`: `sudo apt update && sudo apt install jq`.

---

# Windows Host Notes (Optional)

If you prefer to execute commands from a Windows terminal (PowerShell or Command Prompt) instead of a WSL terminal, you can prefix most Linux commands with `wsl`. For example:

```bash
# From Windows PowerShell/CMD

wsl docker compose up --build -d
wsl docker cp assignment-server-1:/app/data/ca.crt ./data/ca.crt
wsl cd crates/agent && cargo test --test enrollment_test -- --nocapture
```

*   **Pathing**: Be mindful of Windows vs. Linux pathing. While `wsl` can often translate paths, using Linux paths (`/mnt/c/...`) within WSL is generally more robust for file operations.
*   **Environment Variables**: Setting environment variables (like `PUB_KEY`) directly in Windows and passing them to WSL commands can be complex. For multi-line variables, running the full command block inside `wsl bash -c "..."` is often easier.

---

# Long-Term Direction

*   Integrate full backup/storage workflow over the established mTLS channel.
*   Implement server-side mDNS discovery for automated agent discovery.

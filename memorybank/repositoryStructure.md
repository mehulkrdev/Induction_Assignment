# Repository Structure and TDD Execution Strategy

This document outlines the repository structure and the strategy for executing TDD tests for both the Rust agent and Go server components.

## Repository Layout

```text
Assignment/
├── .clinerules.md                # Project-wide architectural and operational guidelines
├── Cargo.toml                    # Rust workspace manifest
├── crates/
│   ├── agent/                    # Rust Agent Project (Windows Native)
│   │   ├── Cargo.toml            # Agent crate manifest
│   │   ├── src/
│   │   │   └── lib.rs            # Core agent logic (key management, enrollment, mTLS client)
│   │   └── tests/
│   │       └── enrollment_test.rs    # Rust integration tests (end-to-end with Go server)
│   │
│   └── agent_logger/             # Rust Logging Crate
│       ├── Cargo.toml            # Logger crate manifest
│       └── src/
│           └── lib.rs            # Rust-specific logging implementation
│
├── server/                       # Go Server Project (WSL/Docker Containerized)
│   ├── Dockerfile                # Defines the Go server's Docker image
│   ├── go.mod                    # Go module definition and dependencies
│   ├── go.sum                    # Checksums for Go module dependencies
│   ├── main.go                   # Server entry point, HTTP/mTLS handlers, CA management
│   ├── pkg/
│   │   ├── certutil/             # Go package for certificate generation, signing, and PEM operations
│   │   └── logger/               # Go package for server-side logging implementation
│   └── tests/
│       └── enrollment_test.go    # Go unit and integration tests for server-side enrollment logic
│
├── docker-compose.yml            # Orchestrates the Go server Docker container
├── README.md                     # Project overview, setup, and execution instructions
├── memorybank/                   # Project documentation and architectural context
│   ├── activeContext.md          # Active development notes and decisions
│   ├── productContext.md         # Product-level requirements and vision
│   ├── projectbrief.md           # High-level project summary
│   ├── repositoryStructure.md    # Details on repository layout and test execution
│   ├── systemPatterns.md         # Core architectural patterns and reliability aspects
│   └── techContext.md            # Technical stack, environments, and execution rules
│
├── bootstrap-bundle.json         # (Optional) Placeholder for future discovery/bootstrap data
├── ca.crt                        # Server's self-signed CA certificate (shared trust anchor)
├── server.crt                    # Server's TLS certificate, signed by CA
├── server.key                    # Server's private key
├── agent.crt                     # Agent's client certificate, signed by server CA
├── agent.key                     # Agent's private key
├── agent.pub                     # Agent's public key
```

## Go Server Test Execution Strategy (Unit Tests and Integration Tests)

*   **Environment**: All Go tests (unit and integration) are executed strictly within isolated Docker containers via WSL.
*   **Command**: `wsl docker compose run --rm server go test -v ./...`
*   **Scope**: Tests within `server/tests/enrollment_test.go` validate server-side logic, including enrollment token validation, public key parsing, and client certificate signing.
*   **Isolation**: The `--rm` flag ensures ephemeral containers, preventing test side-effects from persisting.

## Rust Agent Test Execution Strategy (Integration Tests)

*   **Environment**: Rust tests, particularly integration tests, are executed natively on the Windows host but interact with the Go server running in WSL/Docker.
*   **Workflow for Integration Tests**:
    1.  **Start Go Server**: Initiate the Dockerized Go server in the background: `wsl docker compose up -d server`.
    2.  **Server Readiness**: Wait for a short period (e.g., `wsl sleep 5`) to allow the Go server to initialize and generate its `ca.crt`.
    3.  **Execute Rust Tests**: Run the Rust integration tests which simulate agent behavior: `cargo test --workspace --test enrollment_test`.
        *   These tests are located in `crates/agent/tests/enrollment_test.rs`.
        *   They verify the end-to-end enrollment flow, certificate persistence, and successful mTLS reconnection.
    4.  **Teardown Go Server**: Stop and remove the Docker containers: `wsl docker compose down`.
*   **Trust Context**: The Rust agent tests read the `ca.crt` (copied from the Docker container to the workspace root) to establish trust with the Go server.


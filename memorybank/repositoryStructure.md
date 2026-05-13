# Repository Structure and TDD Execution Strategy

This document outlines the repository structure and the strategy for executing TDD tests for both the Rust agent and Go server components.

## Repository Layout

```
Assignment/
├── Cargo.toml                    # Rust workspace manifest
├── crates/
│   ├── agent/                    # Rust agent project (Windows)
│   │   ├── Cargo.toml            # Agent crate manifest
│   │   ├── src/
│   │   │   └── lib.rs            # Core agent logic and integration tests (Enrollment + mTLS)
│   └── agent_logger/             # Rust agent logging crate
│       ├── Cargo.toml            # Logger crate manifest
│       └── src/
│           └── lib.rs            # Rust logging implementation
│
├── server/                       # Go server project (WSL/Docker)
│   ├── Dockerfile                # Server container definition
│   ├── go.mod                    # Go module definition
│   ├── main.go                   # Server entry point and handlers
│   ├── pkg/
│   │   ├── certutil/             # Certificate generation and signing logic
│   │   └── logger/               # Go server logging implementation
│   └── tests/
│       └── enrollment_test.go    # Go unit tests for enrollment logic
│
├── docker-compose.yml            # Docker orchestration for the server
├── bootstrap-bundle.json         # (Optional) Future discovery data
├── ca.crt                        # Generated CA certificate (Shared trust)
├── server.crt                    # Generated Server Certificate
├── server.key                    # Generated Server Private Key
├── agent.crt                     # Generated Agent Certificate
├── agent.key                     # Generated Agent Private Key
└── memorybank/                   # Project documentation
```

## Docker Build Context and Test Execution Strategy (Go Server)

### **Docker Build Context:**

The `docker-compose.yml` file uses the `./server` directory as its build context. The Go server is built and run entirely within this container.

### **Go Test Execution Strategy (Unit Tests):**

Unit tests are executed in ephemeral containers to ensure isolation.

*   **Command**: `wsl docker compose run --rm server go test -v ./...`
*   **Purpose**: Validates enrollment handlers, JSON parsing, and certificate signing logic without leaving persistent state.

## Rust Agent Test Execution Strategy (Integration Tests)

Integration tests verify the end-to-end flow between the Windows host and the Dockerized server.

### **Command Sequence:**

1.  `wsl docker compose up -d server`
2.  `wsl sleep 5` (Allow server to initialize and generate `ca.crt`)
3.  `cargo test --workspace --test enrollment_test`
4.  `wsl docker compose down`

*   **Context**: The agent reads `ca.crt` from the root directory (shared volume or local copy) to establish trust.
*   **Outcome**: Verifies HTTPS enrollment, certificate persistence, and successful mTLS reconnection on port 8444.

## Port Configuration

- **Port 8443**: HTTPS Enrollment endpoint.
- **Port 8444**: mTLS Secured communication endpoint.

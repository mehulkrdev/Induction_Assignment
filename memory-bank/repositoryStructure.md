# Repository Structure and TDD Execution Strategy

This document outlines the repository structure and the strategy for executing TDD tests for both the Rust agent and Go server components.

## Repository Layout

```text
/Assignment (Root)
├── .clinerules.md                # Project-wide architectural and operational guidelines
├── Cargo.toml                    # Rust workspace manifest
├── crates/
│   ├── agent/                    # Rust Agent Project (WSL Native Execution)
│   │   ├── Cargo.toml            # Agent crate manifest
│   │   ├── src/
│   │   │   └── lib.rs            # Core agent logic (key management, enrollment, mTLS client)
│   │   │   └── main.rs           # Agent executable entry point
│   │   └── tests/
│   │       ├── enrollment_test.rs    # Rust integration tests (end-to-end with Go server)
│   │       └── test_helpers.rs       # Shared Rust test utilities and path resolution
│   │
│   └── agent_logger/             # Rust Logging Crate
│       ├── Cargo.toml            # Logger crate manifest
│       └── src/
│           └── lib.rs            # Rust-specific logging implementation
│
├── data/                         # (Ignored) Ephemeral runtime data (certs, keys) - created at project root
│
├── memory-bank/                   # Project documentation and architectural context
│   ├── activeContext.md          # Active development notes and decisions
│   ├── productContext.md         # Product-level requirements and vision
│   ├── projectbrief.md           # High-level project summary
│   ├── repositoryStructure.md    # Details on repository layout and test execution
│   ├── systemPatterns.md         # Core architectural patterns and reliability aspects
│   └── techContext.md            # Technical stack, environments, and execution rules
│
├── server/                       # Go Server Project (WSL/Docker Containerized)
│   ├── Dockerfile                # Defines the Go server's Docker image
│   ├── go.mod                    # Go module definition and dependencies
│   ├── go.sum                    # Checksums for Go module dependencies
│   ├── main.go                   # Server entry point, HTTP/mTLS handlers, CA management
│   ├── pkg/
│   │   ├── certutil/             # Go package for certificate operations
│   │   ├── enrollment/           # Go package for enrollment service logic
│   │   ├── logger/               # Go package for server-side logging
│   │   └── pathutil/             # Go package for workspace-relative path resolution
│   └── tests/
│       └── enrollment_test.go    # Go unit and integration tests
│
├── third_party/                  # Shared non-source dependencies and assets
│   └── testdata/                 # Reusable integration-test artifacts (e.g., enrollment-agent bundle)
│
├── docker-compose.yml            # Orchestrates the Go server Docker container
└── README.md                     # Project overview, setup, and execution instructions
```

## Go Server Test Execution Strategy (Unit Tests and Integration Tests)

*   **Environment**: All Go tests (unit and integration) are executed strictly within isolated Docker containers in WSL.
*   **Command**: `docker compose run --rm server go test -v ./...` (run from project root in WSL).
*   **Scope**: Tests within `server/tests/enrollment_test.go` validate server-side logic, including enrollment token validation, public key parsing, and client certificate signing.
*   **Isolation**: The `--rm` flag ensures ephemeral containers, preventing test side-effects from persisting.

## Rust Agent Test Execution Strategy (Integration Tests)

*   **Environment**: Rust tests, particularly integration tests, are executed natively within the WSL environment and interact with the Go server running in Docker.
*   **Workflow for Integration Tests (from WSL project root)**:
    1.  **Start Go Server**: Initiate the Dockerized Go server in the background: `docker compose up -d server`.
    2.  **Server Readiness**: Wait for a short period (e.g., `sleep 5`) to allow the Go server to initialize and generate its `ca.crt` in the `data/` directory. (Note: `data/` is at the project root, not inside `server/` or `crates/agent/`).
    3.  **Execute Rust Tests**: Navigate to the agent crate and run its integration tests which simulate agent behavior: `cd crates/agent && cargo test --test enrollment_test -- --nocapture`.
        *   These tests are located in `crates/agent/tests/enrollment_test.rs`.
        *   They use `test_helpers.rs` for standardized workspace-root path resolution to find `third_party/` assets and `data/` artifacts (like `ca.crt`).
        *   They verify the end-to-end enrollment flow, certificate persistence, and successful mTLS reconnection.
    4.  **Teardown Go Server**: Stop and remove the Docker containers from the project root: `docker compose down`.
*   **Trust Context**: The Rust agent tests extract the `ca.crt` from the server`s `data/` directory (copied to the host via `docker cp`) to establish trust.

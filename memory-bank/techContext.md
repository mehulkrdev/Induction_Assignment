# Tech Context

## Languages

- Rust → Agent (Windows)
- Go → Server (runs inside Docker in WSL Debian)

## Environments & Execution Model

### Windows Host (for Rust Agent Development and Execution)
*   **Role**: Primary development and execution environment for both Rust agent and Go server (via Docker).
*   **Execution**: Rust agent binaries and tests run natively within WSL. Go server runs containerized in Docker within WSL.
*   **Tooling**: Uses Cargo for Rust, and Docker/Docker Compose for Go.
*   **Path Context**: All commands (Rust and Docker) are expected to be run from the project root (`Assignment/`) within a WSL terminal. Rust tests automatically resolve the workspace root.

### WSL/Docker (for Go Server Development and Execution)
*   **Role**: Provides a Linux-based isolated environment for the Go server.
*   **WSL (Windows Subsystem for Linux)**:
    *   Hosts the Docker engine and provides a Debian-based Linux environment.
    *   All Go-related operations and Docker commands MUST be executed within WSL.
*   **Docker (Containerization)**:
    *   The Go server runs within a Docker container, ensuring a consistent and isolated runtime.
    *   Go code is never executed directly on the Windows host.
    *   Docker Compose is used for orchestration, defining the server service and its dependencies.

## Tools

- Cargo → Rust build and test
- Docker → Container runtime for Go server
- Go → Only inside Docker container (note: deprecated `io/ioutil` replaced with `os.ReadFile`)
- WSL → Linux environment bridge
- `enrollment_agent_logger` → Rust logging crate (used by agent)
- `github.com/<your-username>/path/to/Assignment/server/pkg/logger` → Go logging package (used by server)
- Reqwest → Rust HTTP client (with `rustls-tls` for HTTPS)
- Serde → Rust serialization/deserialization framework
- `p256`/`pkcs8` → Rust crates for ECDSA P-256 key generation and PEM encoding
- `crypto/tls`, `crypto/x509` → Go standard libraries for TLS and certificate handling

## Execution Rules & Workflow Separation

*   **Rust Agent Workflow (WSL Host)**:
    *   **Build/Test**: `cargo build` or `cargo test` from `crates/agent/` (within WSL).
    *   **Integration with Go Server**: For end-to-end tests, the Go server MUST be running in WSL/Docker before executing Rust integration tests.
        1.  Start Go server: `docker compose up -d server` (from project root in WSL).
        2.  Wait for server readiness (e.g., `sleep 5` for `ca.crt` generation).
        3.  Run Rust integration tests: `cd crates/agent && cargo test --test enrollment_test` (from project root, then `cd` into `crates/agent` in WSL).
        4.  Shut down Go server: `docker compose down` (from project root in WSL).
    *   **Workspace-Root Resolution**: Rust tests use `test_helpers.rs` to find `data/` and `third_party/` relative to the workspace root, making test execution robust to current directory changes within the workspace.

*   **Go Server Workflow (WSL/Docker)**:
    *   **Development/Execution**: All Go code compilation, execution, and testing occurs strictly within Docker containers running in WSL.
    *   **Unit Tests**: `docker compose run --rm server go test -v ./...` (runs `enrollment_test.go` and other unit tests, verifying the `server/pkg/enrollment` package, from project root in WSL).
    *   **Server Startup**: `docker compose up -d server` to start the server in the background (from project root in WSL).
    *   **No Direct Windows Execution**: Go binaries or scripts are never run natively on the Windows host.

## CA-based Trust Model & Cryptographic Ownership Boundaries

*   **Certificate Authority (CA)**:
    *   The Go server acts as its own Certificate Authority, generating a self-signed `ca.crt` upon startup.
    *   This `ca.crt` is the root of trust for both the server's own TLS certificate and the client certificates it issues to agents.
*   **Trust Establishment**:
    *   The Rust agent trusts the server by automatically acquiring and validating the server\\'s certificate against the `ca.crt` during enrollment.
    *   The Go server trusts enrolled agents by validating their client certificates against the same `ca.crt`.
*   **Cryptographic Ownership Boundaries**:
    *   **Rust Agent**: Generates its own ECDSA P-256 keypair (`agent.key`, `agent.pub`). The `agent.key` is strictly owned by the agent and is never transmitted to the server.
    *   **Go Server**: Generates its own CA private key (`ca.key`) and server private key (`server.key`). These private keys are strictly owned by the server and are never exposed to agents.
    *   **Principle**: Neither the agent nor the server ever possesses the private keys of the other component. They only exchange public keys or signed certificates, upholding strong cryptographic ownership boundaries.

## Networking

*   **Server Exposed via Docker (WSL)**:
    *   `localhost:8443` → Secure Enrollment Endpoint (HTTPS)
    *   `localhost:8444` → Mutual TLS (mTLS) Communication Endpoint
*   **Rust Agent Connection**:
    *   Connects to the server using `https://localhost:8443` for enrollment and `https://localhost:8444` for mTLS.
    *   Leverages `rustls-tls` for secure communication.

## Constraints

*   No direct Go installation or execution on Windows host. All Go operations occur within Docker in WSL.
*   Strictly WSL-first execution model: Rust agent runs natively within WSL.
*   All server-side behavior, including unit tests, is validated exclusively within Docker in WSL.
*   Cryptographic private keys (`agent.key`, `ca.key`, `server.key`) are never shared or transmitted across ownership boundaries.
*   Async cleanup in Rust agent tests is handled explicitly to prevent nested Tokio runtime panics.


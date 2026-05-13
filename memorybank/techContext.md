# Tech Context

## Languages

- Rust → Agent (Windows)
- Go → Server (runs inside Docker in WSL Debian)

## Environments

### Windows (Host)
- Runs Rust agent
- Uses Cargo for build and test
- Path: `crates/agent/`

### WSL Debian
- Linux environment inside Windows
- Hosts Docker engine

### Docker (inside WSL)
- Runs Go server container
- Provides isolated Linux runtime

## Tools

- Cargo → Rust build and test
- Docker → Container runtime for Go server
- Go → Only inside Docker container
- WSL → Linux environment bridge
- `enrollment_agent_logger` → Rust logging crate (used by agent)
- `github.com/mehulkrdev/Assignment/server/pkg/logger` → Go logging package (used by server)
- Reqwest → Rust HTTP client (with `rustls-tls` for HTTPS)
- Serde → Rust serialization/deserialization framework
- `p256`/`pkcs8` → Rust crates for ECDSA P-256 key generation and PEM encoding
- `crypto/tls`, `crypto/x509` → Go standard libraries for TLS and certificate handling

## Execution Rules

- Rust commands (e.g., `cargo build`, `cargo test`) MUST be executed from the `crates/agent/` directory on the Windows host, or by using `cargo test --workspace` from the repository root.
- Go code MUST NOT be executed directly on the Windows host. All Go server operations, including unit tests, must occur within a Docker container.
- Go Unit Tests: Run using ephemeral containers via `wsl docker compose run --rm server go test -v ./...`.
- Rust Integration Tests: These tests verify the end-to-end flow. The command sequence is:
    1.  Start the Go server: `wsl docker compose up -d server`
    2.  Allow server initialization: `wsl sleep 5` (ensures `ca.crt` is generated)
    3.  Execute Rust tests: `cargo test --workspace` (runs tests within `crates/agent/src/lib.rs`)
    4.  Tear down server: `wsl docker compose down`
- All Go server tests (`enrollment_test.go`) MUST run inside the Docker container.
- Docker commands MUST be executed within the WSL Debian environment.

## Networking

- Server exposed via Docker:
  localhost:8443 → Enrollment (HTTPS)
  localhost:8444 → mTLS communication

- Rust agent connects using:
  https://localhost:8443

## Constraints

- No Go installation required on Windows
- No cross-environment command mixing
- All server behavior validated via Docker
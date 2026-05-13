# Tech Context

## Languages

- Rust → Agent (Windows)
- Go → Server (runs inside Docker in WSL Debian)

## Environments

### Windows (Host)
- Runs Rust agent
- Uses Cargo for build and test
- Path: `agent/`

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
- Reqwest → HTTP client for Rust agent (with `rustls-tls` for HTTPS)
- Chrono/OnceCell → Rust logging dependencies (centralized under `logger/client`)
- Go logger → Go logging implementation (centralized under `logger/server`)
- p256/pkcs8 → Rust crates for ECDSA P-256 key generation and PEM encoding
- reqwest → Rust HTTP client with `rustls-tls` for secure communication
- crypto/tls, crypto/x509 → Go standard libraries for TLS and certificate handling

## Execution Rules

- Rust commands (cargo build/test) MUST run on Windows inside agent folder
- Go code MUST NOT be executed directly on Windows
- Go Unit Tests: Use ephemeral containers (`docker compose run --rm server go test -v ./...`)
- Rust Integration Tests: Use persistent container (`docker compose up -d server`) and ensure `wsl sleep 5` before execution.
- Go tests MUST run inside Docker container only
- Docker commands run inside WSL Debian

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
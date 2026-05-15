# System Patterns

## Architecture Overview

The system follows a client-server architecture with strict environment separation and modular service organization.

**Windows Host (Client)**
└── Rust Agent (`crates/agent`)
    ├── `src/main.rs`: CLI entry point.
    └── `src/lib.rs`: Core agent logic.

**WSL Debian (Infrastructure)**
└── Docker
    └── Go Server (Appliance)
        ├── `main.go`: Server initialization.
        └── `pkg/enrollment`: Modular enrollment service.

## Communication Flow

1. **Enrollment (HTTPS)**: Agent (Windows) → `https://localhost:8443/enroll` → Go Server (Docker)
2. **Secure Communication (mTLS)**: Agent (Windows) → `https://localhost:8444/secure` → Go Server (Docker)

## Key Design Patterns

### 1. Client-Server Pattern
*   Rust agent acts as the client.
*   Go server acts as the appliance/server.
*   Communication uses standard HTTP/TLS protocols.

### 2. Enrollment Pattern (Certificate-based Trust Bootstrapping)
*   **Initial Trust (Bootstrapping)**: The Rust agent establishes trust by reading the server-generated `ca.crt`. This root certificate is used to validate the server's certificate during HTTPS enrollment.
*   **Agent Identity Generation**: Upon initiating enrollment, the agent generates a cryptographically secure ECDSA P-256 keypair (`agent.key` and its corresponding public key).
*   **Secure Enrollment Request**: The agent sends a POST request to `https://localhost:8443/enroll`. The request body includes an `enrollment_token` (for server authorization), `agent_id`, and the PEM-encoded `public_key`.
*   **Server Validation and Signing**: The Go server validates the `enrollment_token` using the modular enrollment service in `server/pkg/enrollment`. If authorized, it signs the agent's public key using its internal Certificate Authority (CA) and returns a PEM-encoded client certificate (`agent.crt`).
*   **Certificate Persistence**: The agent securely persists both its private key (`agent.key`) and the issued client certificate (`agent.crt`) locally for subsequent mTLS communication.

### 3. Mutual TLS (mTLS) Pattern (Bidirectional Authentication)
*   **mTLS Reconnection**: After successful enrollment, the agent uses its persisted client certificate (`agent.crt`) and private key (`agent.key`) to establish a mutual TLS connection with the server on port `8444`.
*   **Bi-directional Verification**:
    *   **Server-side**: The server mandates and verifies the client certificate presented by the agent against its trusted `ca.crt` pool.
    *   **Agent-side**: The agent verifies the server's certificate using the same `ca.crt`.
*   **Authenticated Communication**: Successful mTLS negotiation ensures bidirectional identity verification and establishes a secure channel for authorized communication to protected endpoints like `/secure`.

### 4. Environment Isolation Pattern
*   The Rust agent executes natively on the Windows host.
*   The Go server runs in an isolated Docker container within the WSL Debian environment.
*   Strict separation: No direct execution of Go code on Windows or Rust code within the Docker environment.

### 5. Centralized Logging Pattern
*   Standardized logging format (`DD-MM-YYYY_HH:MM`) is enforced across both Rust and Go components.
*   Dedicated `logger/` directory within each component (`crates/agent_logger/`, `server/pkg/logger/`) provides language-specific logging implementations.

## Runtime Handling Improvements, Reliability, and Operational Clarity

### Error Handling and Resilience Strategy
*   **TLS Handshake Failures**: The system is designed to gracefully handle TLS handshake failures (e.g., due to certificate mismatches, expired certificates, or untrusted CAs). Specific error messages and logging are implemented to aid in diagnosis.
*   **Malformed Enrollment Requests/Responses**: The server validates incoming enrollment request payloads (JSON structure, `enrollment_token`, `agent_id`, `public_key`). Malformed requests result in appropriate HTTP error responses (e.g., 400 Bad Request) and detailed server-side logging. The agent is expected to handle unexpected or malformed responses from the server robustly.
*   **Filesystem/Permission Failures**: Both the agent (for persisting `agent.key`, `agent.pub`, `agent.crt`) and the server (for `ca.key`, `ca.crt`, `server.key`, `server.crt`) incorporate robust error handling for filesystem operations. This includes checking for directory permissions, disk space, and file corruption during read/write operations. Critical errors are logged and, if unrecoverable, lead to graceful shutdown.
*   **Corrupted/Missing PEM Files**: The system includes validation checks when loading PEM-encoded keys and certificates. If a file is corrupted or missing, the component will log the error and prevent proceeding with cryptographic operations, avoiding unexpected runtime behavior.
*   **Key/Certificate Mismatch Validation**: During mTLS, the agent ensures that the private key it uses matches the public key embedded in its client certificate. Similarly, the server validates that the client certificate presented matches the cryptographic proof of ownership provided by the agent's private key.

### Retry and Backoff Strategy
*   **Connection Failures**: For transient network issues or temporary server unavailability (e.g., during startup), the agent implements a retry mechanism with an exponential backoff strategy for enrollment requests and mTLS connection attempts.
*     **Initial Delay**: A small initial delay (e.g., 1-5 seconds).
*     **Exponential Backoff**: Subsequent retries increase the delay exponentially (e.g., 2^n seconds).
*     **Jitter**: Random jitter is applied to backoff delays to prevent synchronized retries from overwhelming the server.
*     **Max Retries/Timeout**: A configured maximum number of retries or a total timeout period is enforced to prevent indefinite blocking.

### Test Organization and Operational Guidelines
*   **Test Organization**: Aligns with a TDD-oriented structure with enhanced isolation and helpers:
    *   **Rust Agent Tests**: Located in `crates/agent/tests/`. Utilizes `test_helpers.rs` for mocked server interactions and isolated environment setup. Focus on end-to-end enrollment, certificate persistence, and mTLS reconnection behavior.
    *   **Go Server Tests**: Located in `server/tests/`. Focus on unit testing enrollment endpoint validation, token validation, and certificate signing logic within `pkg/enrollment`. All Go tests run inside Docker.
*   **Operational Guidelines**:
    *   **Traceability**: Comprehensive logging (`DD-MM-YYYY_HH:MM`) at critical points for auditing and debugging.
    *   **Monitoring**: Key operational metrics (e.g., successful enrollments, mTLS connection rates, error counts) are exposed for monitoring (future enhancement).
    *   **Maintainability**: Adherence to established coding standards and project patterns (as per `.clinerules.md`) ensures code readability and ease of maintenance.
    *   **Reproducibility**: Dockerized server environment and defined test execution commands ensure consistent and reproducible behavior across development environments.

## Communication Stages

### Stage 1: Discovery (Planned)
*   Agent discovers server via mDNS (future enhancement).

### Stage 2: Verification (Current)
*   Agent validates server identity using `ca.crt` (Bootstrap trust).

### Stage 3: Enrollment (Implemented)
*   Agent initiates an HTTPS connection to port `8443`.
*   Agent generates an ECDSA P-256 keypair.
*   Agent sends a POST request with `enrollment_token`, `agent_id`, and its PEM-encoded public key.
*   Server validates the token and signs the public key with its internal CA via the enrollment service.

### Stage 4: Certificate Persistence (Implemented)
*   Agent receives the signed certificate from the server.
*   Agent persists both the private key (`agent.key`) and the certificate (`agent.crt`) to the local file system.

### Stage 5: Secure mTLS Communication (Implemented)
*   Agent establishes a new connection to port `8444` using its mTLS identity (`agent.crt`, `agent.key`).
*   Server mandates and verifies the client certificate.
*   Agent verifies the server certificate using `ca.crt`.
*   Secure, bi-directionally authenticated communication is established.

## Constraints

*   Strict TDD (tests first).
*   Modular Service Design (separation of concerns).
*   Minimal implementation initially.
*   Clear separation of responsibilities.
*   No mixing of Windows and WSL execution environments.
*   No cryptographic private keys are ever transmitted or shared across component boundaries.

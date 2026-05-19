# System Architecture

## Overview

The system is a secure, automated enrollment and authenticated communication framework designed for zero-trust environments. It facilitates a transition from an unauthenticated state to a fully verified, bi-directionally authenticated mutual TLS (mTLS) communication channel between a native Windows agent and a containerized Go server.

### Environment Topology

The architecture leverages strict environment isolation to enhance security and operational consistency:

*   **Windows Host (Agent Domain)**: Executes the Rust-based Agent natively. This environment handles local persistence of cryptographic identities and coordinates communication with the server.
*   **WSL/Docker (Server Domain)**: Hosts the Go-based Server (Appliance) within a Debian-based Linux environment. This domain acts as the Certificate Authority (CA) and orchestrates the secure endpoints.

---

## System Context Diagram

```
+-----------------------+
|   Windows Host        |
| +-------------------+ |
| |    Rust Agent     | |
| | (Native Execution)| |
| +---------+---------+ |
|           | HTTPS (8443)  |
|           | mTLS (8444)   |
+-----------|-------------+
            |
            | (WSL Network Boundary)
            |
+-----------|-------------+
|   WSL Debian (Linux)  |
| +-------------------+ |
| |      Docker       | |
| | +---------------+ | |
| | |   Go Server   | | |
| | | (Containerized)| | |
| | +---------------+ | |
| +-------------------+ |
+-----------------------+
```

---

## Architecture Principles

This system adheres to the following core architectural principles:

1.  **Zero Trust**: All entities, whether inside or outside the network perimeter, must be authenticated and authorized before gaining access to resources. This is enforced through a multi-stage enrollment and mTLS authentication process.
2.  **Environment Isolation**: Components are strictly isolated. The Rust agent runs natively on Windows, while the Go server is containerized within a WSL-hosted Docker environment. This separation minimizes attack surface and ensures consistent execution environments.
3.  **Cryptographic Ownership Boundaries**: Private keys (`agent.key`, `ca.key`, `server.key`) are never transmitted or shared across component boundaries. Agents generate their own private keys, and the server acts solely as a Certificate Authority to sign public keys.
4.  **Test-Driven Development (TDD)**: All features and critical functionalities are developed with a "tests first" approach, ensuring robust validation, maintainability, and early detection of defects.
5.  **Secure by Design**: Cryptographic best practices (ECDSA P-256, HTTPS, mTLS) are fundamental to the system design, not an afterthought. Default configurations prioritize security.
6.  **Auditability & Observability**: Standardized logging provides clear, timestamped records of critical events for auditing, debugging, and operational monitoring.

---

## Package Architecture

### Rust Agent (`crates/agent`)

*   `src/lib.rs`: Contains the core `Agent` struct and its associated methods for key generation, enrollment requests, and mTLS reconnection. It also defines `EnrollmentRequest` and `EnrollmentResponse` data structures. Traits for `Discovery` and `EnrollmentClient` are defined, enabling mocking for testing.
*   `src/main.rs`: (Implied or to be created for executable) The entry point for the agent application.
*   `tests/enrollment_test.rs`: Integration tests for the agent, verifying end-to-end enrollment and reconnection flows.

### Rust Agent Logger (`crates/agent_logger`)

*   `src/lib.rs`: Provides a centralized logging utility for the Rust agent, handling log file creation, rotation, and macro-based logging (`log_entry!`). Utilizes `once_cell` for safe global state management.

### Go Server (`server`)

*   `main.go`: The server's entry point, responsible for:
    *   Initializing the internal CA (`certutil.GenerateCACert`).
    *   Generating server TLS certificates (`certutil.GenerateServerCert`).
    *   Configuring and starting the HTTPS enrollment server (Port 8443) and mTLS secure server (Port 8444).
    *   Implementing the `EnrollmentHandler` logic.
*   `pkg/certutil/certutil.go`: Encapsulates all certificate management functionalities:
    *   `GenerateCACert`: Creates a self-signed root CA certificate and private key.
    *   `GenerateServerCert`: Generates server certificates signed by the internal CA.
    *   `SignClientPublicKey`: Signs an agent's public key to issue a client certificate.
    *   PEM encoding/decoding utilities.
    *   `PublicKeyError`: Custom error type for public key parsing issues.
*   `pkg/logger/logger.go`: Provides a synchronized logging utility for the Go server, supporting multi-writer output (file and stdout).
*   `tests/enrollment_test.go`: Unit tests for the server's `EnrollmentHandler` and `certutil` package, covering request validation, token checks, and certificate signing.

---


## Component Responsibilities

### Rust Agent
The Agent is the active initiator in the architecture, responsible for:
*   **Trust Bootstrapping**: Verifying the server's identity using a pre-distributed or discovered `ca.crt`.
*   **Identity Generation**: Creating local ECDSA P-256 keypairs.
*   **Enrollment Orchestration**: Managing the multi-step enrollment lifecycle, including token-based authentication and certificate acquisition.
*   **Secure Persistence**: Storing the issued client certificate (`agent.crt`) and private key (`agent.key`) securely on the host filesystem.
*   **mTLS Client**: Initiating and maintaining authenticated connections to the server's secure management ports.
*   **Resilience**: Implementing retry logic with exponential backoff for network-bound operations.

### Go Server (Appliance)
The Server acts as the central authority and secure gateway, responsible for:
*   **Internal CA Management**: Generating and managing the root Certificate Authority (CA) for the system.
*   **Certificate Issuance**: Validating enrollment tokens and signing agent public keys to issue client certificates.
*   **Secure Endpoint Hosting**:
    *   **Port 8443**: HTTPS enrollment endpoint for bootstrapping identities.
    *   **Port 8444**: Strict mTLS endpoint requiring valid client certificates for all requests.
*   **Agent Verification**: Extracting and validating agent identities from presented TLS certificates during mTLS handshakes.
*   **Containerized Execution**: Ensuring a reproducible and isolated runtime via Docker.

---

## Critical Workflows

### 1. Identity Enrollment Flow (Trust Bootstrapping)
The enrollment process follows a strictly defined sequence to ensure no private keys are ever transmitted across the network:

1.  **Bootstrap**: The Agent reads `ca.crt` to establish a root of trust.
2.  **Key Gen**: Agent generates an ECDSA P-256 keypair locally.
3.  **Request**: Agent sends a JSON-encoded POST request to `https://localhost:8443/enroll` containing its `agent_id`, `enrollment_token`, and PEM-encoded `public_key`.
4.  **Verification**: Server validates the `enrollment_token` (e.g., checks against internal policy/static mapping).
5.  **Signing**: Server uses its CA private key to sign the Agent's public key, creating a client certificate.
6.  **Response**: Server returns the signed certificate in a JSON response.
7.  **Persistence**: Agent saves `agent.crt` and `agent.key` to the local filesystem.

### 2. Mutual TLS (mTLS) Communication
Once enrolled, all subsequent communication occurs over port 8444:

*   **Handshake**: The Agent initiates a TLS handshake presenting its `agent.crt` and proving ownership via `agent.key`.
*   **Bi-directional Trust**:
    *   **Agent** verifies the Server's certificate against `ca.crt`.
    *   **Server** verifies the Agent's certificate against `ca.crt` and ensures it contains the correct `CommonName` (AgentID).
*   **Secure Channel**: Upon successful handshake, a secure, encrypted, and bi-directionally authenticated channel is established.

---

## Data Flow & Communication Patterns

### Protocols
*   **Control Plane (Enrollment)**: HTTPS over TCP (Port 8443).
*   **Data/Management Plane (Operational)**: mTLS over TCP (Port 8444).

### Data Formats
*   **API Payloads**: JSON (Standardized across Go and Rust via `serde` and `encoding/json`).
*   **Cryptographic Artifacts**: PEM-encoded X.509 certificates and PKCS#8/EC private keys.

---

## Data Flow — Agent Enrollment

1.  **Agent Initialization**: The Rust Agent starts, reads the `ca.crt` to establish trust for the server.
2.  **Keypair Generation**: Agent generates a new ECDSA P-256 private key and derives the public key locally. The private key remains on the agent.
3.  **Enrollment Request (Agent -> Server)**: Agent constructs an `EnrollmentRequest` (JSON) containing `agent_id`, `enrollment_token`, and the PEM-encoded public key. This is sent via `POST` to `https://localhost:8443/enroll`.
4.  **Server Validation**: The Go Server receives the request, validates the `enrollment_token` and `agent_id`, and parses the provided public key.
5.  **Certificate Signing (Server Internal)**: The Server, acting as a CA, uses its `ca.key` to sign the Agent’s public key, generating a client X.509 certificate (`agent.crt`).
6.  **Enrollment Response (Server -> Agent)**: Server sends an `EnrollmentResponse` (JSON) back to the agent, containing a "success" status and the PEM-encoded `agent.crt`.
7.  **Key and Certificate Persistence**: Agent receives `agent.crt` and saves both `agent.crt` and its corresponding `agent.key` to the local filesystem for future mTLS use.

---

## Data Flow — mTLS Reconnection

1.  **Agent Reconnection Attempt**: The Rust Agent attempts to establish a connection to the server on the mTLS port `https://localhost:8444/secure`.
2.  **Load Identity**: Agent loads its persisted `agent.key` and `agent.crt` from the local filesystem.
3.  **mTLS Handshake (Agent & Server)**:
    *   **Client Hello**: Agent sends its `agent.crt` to the server.
    *   **Server Hello**: Server sends its `server.crt` to the agent.
    *   **Certificate Verification (Agent)**: Agent verifies `server.crt` using the trusted `ca.crt`.
    *   **Certificate Verification (Server)**: Server verifies `agent.crt` using the trusted `ca.crt`. It also validates the `CommonName` in `agent.crt` against expected agent identifiers.
    *   **Key Exchange**: Cryptographic keys are exchanged securely, leveraging ECDHE ciphers.
4.  **Authenticated Request/Response**: Upon successful handshake, a mutually authenticated and encrypted channel is established. Agent sends requests (e.g., to `/secure`), and the server responds, knowing the agent's identity is verified.

---

## Security Architecture

### Cryptographic Foundation
*   **Key Generation**: Agents and the Server utilize ECDSA P-256 for all keypair generation, providing strong cryptographic security with smaller key sizes compared to RSA.
*   **Certificate Authority (CA)**: The Go server operates a self-signed, ephemeral CA. This CA is generated at server startup, issuing certificates for the server itself and signing agent public keys during enrollment.
*   **X.509 Certificates**: Standard X.509 certificates are used for identity and trust chaining, enabling verification via `ca.crt`.

### Trust Management
*   **Initial Trust**: The `ca.crt` acts as the root of trust, distributed to agents out-of-band or discovered. It is used to verify the authenticity of the server during the initial HTTPS enrollment and subsequently for mTLS.
*   **Enrollment Token**: A shared secret (`enrollment_token`) provides an initial layer of authentication for enrollment requests, preventing unauthorized agents from obtaining certificates.
*   **Mutual TLS (mTLS)**: Enforces bi-directional authentication. Both the agent and server present and verify each other's certificates, ensuring only trusted parties can communicate on critical channels.

### Key Management & Protection
*   **Private Key Never Transmitted**: Agent private keys (`agent.key`) are generated and stored locally on the agent and are never sent over the network. Similarly, the CA private key (`ca.key`) and server private key (`server.key`) remain server-side.
*   **Secure Storage**: Private keys and certificates are persisted to disk with appropriate permissions (e.g., `0600` for private keys) to restrict unauthorized access.
*   **Ephemeral CA**: The CA is re-generated on server restart, ensuring that if a CA private key were compromised, its impact would be limited to the current operational period.

---

## Key Architectural Decisions (KADs)

1.  **Decision: Use of `rustls-tls` in Rust Agent over OpenSSL.**
    *   **Context**: The Rust ecosystem offers multiple TLS backends, including `rustls` and bindings to `OpenSSL`.
    *   **Decision**: `rustls` was chosen for its pure Rust implementation, which simplifies dependency management, reduces exposure to C/C++ vulnerabilities (common in OpenSSL), and often provides better cross-platform compatibility without system library dependencies.
    *   **Implication**: Slightly higher compile times due to pure Rust cryptography, but improved security posture and reduced operational overhead.

2.  **Decision: Self-Signed, Ephemeral CA for the Go Server.**
    *   **Context**: The system requires a mechanism to issue and verify certificates for agents. Options include using an external CA, a persistent internal CA, or an ephemeral internal CA.
    *   **Decision**: An ephemeral, self-signed CA is generated at each server startup. This CA issues the server's own certificate and signs client certificates.
    *   **Implication**: Simplifies deployment (no external CA management), enhances security by limiting the lifespan of the CA's private key (reducing impact of compromise to a single runtime), but requires agents to re-establish trust with the new `ca.crt` on server restart.

3.  **Decision: Strict Environment Isolation (Windows Native Agent, WSL/Docker Go Server).**
    *   **Context**: The system operates across two distinct host environments (Windows for the agent, Linux for the server).
    *   **Decision**: Enforce strict isolation where Rust binaries run natively on Windows, and Go binaries are containerized within WSL/Docker. No direct execution of Go on Windows or Rust within Docker.
    *   **Implication**: Guarantees consistent runtime environments, simplifies development/deployment by leveraging native tooling (Cargo on Windows, Docker in WSL), and enhances security by compartmentalizing potential attack vectors.

4.  **Decision: Exponential Backoff with Retries for Agent Communications.**
    *   **Context**: Network communications are inherently unreliable, especially during initial setup or transient server unavailability.
    *   **Decision**: The Rust agent implements a retry mechanism with exponential backoff (and jitter) for both enrollment and mTLS reconnection attempts.
    *   **Implication**: Improves system resilience and fault tolerance, ensuring the agent can recover from temporary network glitches or server restarts without manual intervention. Prevents overwhelming the server with constant retries.

5.  **Decision: Separate Ports for Enrollment (HTTPS) and mTLS Communication.**
    *   **Context**: Both enrollment and secure communication involve TLS, but serve distinct purposes and security requirements.
    *   **Decision**: Use `8443` for the initial HTTPS enrollment (allowing a bootstrapped agent to get a cert) and `8444` for strict mTLS authenticated communication (requiring a valid client certificate).
    *   **Implication**: Enforces a clear separation of concerns, allowing the enrollment port to have more permissive (though still secure) authentication requirements for initial bootstrapping, while the operational port maintains maximum security with bi-directional client certificate validation.


## Storage & Persistence

### Agent Persistence (Windows Host)
*   `ca.crt`: Trusted root CA certificate.
*   `agent.crt`: Issued client identity certificate.
*   `agent.key`: Private key (Permissions: 0600 equivalent on Windows).

### Server Persistence (Docker Container)
*   `ca.crt` / `ca.key`: Root CA artifacts generated at runtime.
*   `server.crt` / `server.key`: Server-specific identity artifacts signed by internal CA.

---

## Logging Architecture

The system implements a centralized logging pattern to ensure auditability across distributed components:

*   **Standardized Format**: `DD-MM-YYYY_HH:MM` timestamping.
*   **Implementation**:
    *   **Rust**: Custom `agent_logger` crate using `once_cell` for global file handle management and `log_entry!` macros.
    *   **Go**: Standard library `log` package wrapped in a custom `pkg/logger` for multi-writer support (Stdout + File).
*   **Storage**: Logs are stored in a structured directory format (e.g., `Logs/{timestamp}/{component}.log` relative to the workspace root).

---

## Testing Structure

The architecture is strictly validated through a TDD (Test-Driven Development) approach:

*   **Unit/Component Tests**:
    *   **Go**: Logic for certificate signing and endpoint validation is tested within the Docker container (`go test`).
    *   **Rust**: Agent logic and state transitions are tested via Cargo (`cargo test`).
*   **Integration Tests**:
    *   Verify the end-to-end flow from enrollment to mTLS reconnection.
    *   Requires a running Go server instance (via Docker Compose) and execution of the Rust test suite on the host.
*   **Mocking**: Rust tests leverage `mockall` for isolating external dependencies (e.g., Discovery, EnrollmentClient).

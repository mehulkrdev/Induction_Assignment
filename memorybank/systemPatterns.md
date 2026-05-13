# System Patterns

## Architecture Overview

The system follows a client-server architecture with strict environment separation.

**Windows Host (Client)**
└── Rust Agent

**WSL Debian (Infrastructure)**
└── Docker
    └── Go Server (Appliance)

## Communication Flow

1. **Enrollment (HTTPS)**: Agent (Windows) → `https://localhost:8443/enroll` → Go Server (Docker)
2. **Secure Communication (mTLS)**: Agent (Windows) → `https://localhost:8444/secure` → Go Server (Docker)

## Key Design Patterns

### 1. Client-Server Pattern
- Rust agent acts as the client.
- Go server acts as the appliance/server.
- Communication uses standard HTTP/TLS protocols.

### 2. Enrollment Pattern
- **Initial Trust (Bootstrapping)**: The Rust agent established trust by reading the server-generated `ca.crt`. This root certificate is used to validate the server's certificate during HTTPS enrollment.
- **Agent Identity Generation**: Upon starting enrollment, the agent generates an ECDSA P-256 keypair (`agent.key` and its corresponding public key).
- **Secure Enrollment Request**: The agent sends a POST request to `https://localhost:8443/enroll`. The request body includes the `enrollment_token`, `agent_id`, and the PEM-encoded `public_key`.
- **Server Validation and Signing**: The Go server validates the enrollment token. If valid, it signs the agent's public key using the internal CA and returns a PEM-encoded client certificate (`agent.crt`).
- **Certificate Persistence**: The agent persists both the private key (`agent.key`) and the issued certificate (`agent.crt`) locally for subsequent mTLS communication.

### 3. Mutual TLS (mTLS) Pattern
- **mTLS Reconnection**: After successful enrollment, the agent uses its persisted `agent.crt` and `agent.key` to establish a mutual TLS connection with the server on port 8444.
- **Bi-directional Verification**:
    - **Server-side**: The server requires a client certificate and verifies it against the `ca.crt` pool.
    - **Agent-side**: The agent verifies the server's certificate using the same `ca.crt`.
- **Authenticated Communication**: Successful mTLS negotiation allows the agent to access protected endpoints like `/secure`.

### 4. Environment Isolation Pattern
- Agent runs on Windows.
- Server runs inside Docker (WSL Debian).
- No direct execution of Go code on Windows or Rust code in Docker.

### 5. Centralized Logging Pattern
- Standardized logging format (`DD-MM-YYYY_HH:MM`) across languages.
- Centralized `logger/` directory with language-specific implementations.

## Communication Stages

### Stage 1: Discovery (Planned)
- Agent discovers server via mDNS.

### Stage 2: Verification (Current)
- Agent validates server identity using `ca.crt` (Bootstrap trust).

### Stage 3: Enrollment (Implemented)
- Agent initiates an HTTPS connection to port 8443.
- Agent generates an ECDSA P-256 keypair.
- Agent sends a POST request with `enrollment_token`, `agent_id`, and its PEM-encoded public key.
- Server validates the token and signs the public key with its internal CA.

### Stage 4: Certificate Persistence (Implemented)
- Agent receives the signed certificate from the server.
- Agent persists both the private key (`agent.key`) and the certificate (`agent.crt`) to the local file system.

### Stage 5: Secure mTLS Communication (Implemented)
- Agent establishes a new connection to port 8444 using its mTLS identity (`agent.crt`, `agent.key`).
- Server mandates and verifies the client certificate.
- Agent verifies the server certificate using `ca.crt`.
- Secure, bi-directionally authenticated communication is established.

## Constraints

- Strict TDD (tests first).
- Minimal implementation initially.
- Clear separation of responsibilities.
- No mixing of Windows and WSL execution environments.

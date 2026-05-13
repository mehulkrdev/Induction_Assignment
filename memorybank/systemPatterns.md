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
- Initial trust bootstrapping via `ca.crt`.
- HMAC-based token authentication for enrollment.
- ECDSA P-256 keypair generation on the agent.
- Server-side public key signing and certificate issuance.

### 3. Mutual TLS (mTLS) Pattern
- Ongoing communication requires valid client and server certificates.
- Server validates client certificate against the internal CA.
- Agent validates server certificate using the same CA.

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
- Agent connects to port 8443 via HTTPS.
- Agent sends enrollment request with HMAC token and public key.
- Server validates token and signs the public key.

### Stage 4: Certificate Persistence (Implemented)
- Agent saves the issued certificate (`agent.crt`) and private key (`agent.key`).

### Stage 5: Secure mTLS Communication (Implemented)
- Agent reconnects using the persisted identity on port 8444.
- Server verifies client certificate and grants access to `/secure`.

## Constraints

- Strict TDD (tests first).
- Minimal implementation initially.
- Clear separation of responsibilities.
- No mixing of Windows and WSL execution environments.

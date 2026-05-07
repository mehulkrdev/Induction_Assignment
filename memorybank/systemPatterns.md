# System Patterns

## Architecture Overview

The system follows a client-server architecture with strict environment separation.

Windows Host
└── Rust Agent (Client)

WSL Debian
└── Docker
    └── Go Server (Appliance)

## Communication Flow

Agent (Windows) → localhost:8443 → Docker Container (Go Server)

## Key Design Patterns

### 1. Client-Server Pattern
- Rust agent acts as client
- Go server acts as appliance/server
- Communication over HTTP/TLS

### 2. Enrollment Pattern
- One-time enrollment phase
- Token-based authentication (HMAC-SHA256)
- Certificate issuance after validation

### 3. Trust Establishment Pattern
- BEB provides initial trust (CA fingerprint)
- TLS used for secure enrollment
- mTLS used for ongoing communication

### 4. Environment Isolation Pattern
- Agent runs on Windows
- Server runs inside Docker (WSL Debian)
- No direct execution of Go code on Windows

## Communication Stages

### Stage 1: Discovery
- Agent discovers server via mDNS

### Stage 2: Verification
- Agent validates server identity using BEB

### Stage 3: Enrollment
- Agent connects to port 8443
- Sends enrollment request with token

### Stage 4: Certificate Issuance
- Server validates token
- Issues client certificate

### Stage 5: Secure Communication
- All further communication over mTLS (port 8444)

## Constraints

- Strict TDD (tests first)
- Minimal implementation initially
- Clear separation of responsibilities
- No mixing of Windows and WSL execution environments
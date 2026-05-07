# Product Context

## Why this project exists
This project simulates a real-world secure backup/storage system where a client (agent) must establish trust with a server before any data communication.

## Problems it solves

- Establishing trust between unknown client and server
- Preventing unauthorized access during enrollment
- Ensuring all communication is encrypted and authenticated
- Automating certificate issuance and management

## How it should work

1. Agent starts with no trust
2. Uses BEB (Bootstrap Enrollment Bundle) to verify server identity
3. Connects to server securely over TLS (port 8443)
4. Sends enrollment request with HMAC-based token
5. Server validates and issues client certificate
6. All future communication switches to mTLS (port 8444)

## User Experience Goals

- Zero manual configuration
- Fully automated enrollment
- Secure by default
- Minimal user intervention
- Clear separation between enrollment and communication phases

## Key Principles

- Security first (no shortcuts in trust establishment)
- Simplicity in flow
- Testability (TDD-driven design)
- Clear client-server responsibilities
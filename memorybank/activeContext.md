# Active Context

## Current Focus

- Maintenance and verification of the secure enrollment and mTLS communication system.
- Ensuring strict environment separation (WSL-first workflow) and robust error handling during the enrollment flow.
- Maintaining separate logging implementations for Rust and Go to ensure clean language boundaries.
- Implementing robust workspace-root path resolution for tests and runtime artifacts.
- Ensuring explicit async cleanup in Rust tests to prevent runtime panics.

## Current State

- **Modularized Enrollment Service**:
    - Go server logic moved to `server/pkg/enrollment/service.go` for better separation of concerns.
    - Rust agent enhanced with a clear `main.rs` entry point and refined `lib.rs`.
- **Fully implemented** secure enrollment logic:
    - Rust agent generates ECDSA P-256 keypairs.
    - Go server validates HMAC-based tokens and signs agent public keys.
    - Agent persists the issued certificate and private key.
- **Fully implemented** mTLS communication:
    - Agent uses its certificate and key for authenticated requests on port 8444.
    - Go server verifies agent identity via mTLS.
- **Enhanced Testing Infrastructure**:
    - Added `crates/agent/tests/test_helpers.rs` for streamlined integration testing with robust workspace-root resolution.
    - Improved test isolation across Go and Rust suites, including explicit async cleanup.
- **Successful integration tests**:
    - Rust integration tests (`enrollment_test.rs`) verify the complete flow: Enrollment -> Persistence -> Reconnection (mTLS).
    - Go unit tests (`enrollment_test.go`) verify endpoint validation and certificate signing logic.
- **Isolated Logging**: Decentralized logging system implemented with separate Go (`server/pkg/logger`) and Rust (`crates/agent_logger`) implementations, maintaining identical log formats and directory structures.
- **Runtime-Generated TLS Model**: Server dynamically generates CA and TLS certificates on startup, with clear instructions for `ca.crt` extraction.

## Known Issues

## Immediate Next Steps

1. Maintain and monitor the system for any edge cases in certificate handling, especially concerning ephemeral CA lifecycles.
2. Ensure any new features follow the established TDD patterns and the WSL-first execution model.
3. Continue refining workspace-root resolution for all runtime artifacts.

## Completed Milestones

- [x] Agent connection test
- [x] Basic enrollment request test with token validation
- [x] Basic enrollment request implementation
- [x] Secure HTTPS enrollment with ECDSA P-256 certificate issuance and persistence
- [x] mTLS communication implementation (Port 8444)
- [x] Integration of centralized logging system
- [x] Full end-to-end integration test (Enrollment + mTLS)
- [x] Resolved incorrect cross-language module integration between Rust and Go.
- [x] Modularized Go enrollment logic into `server/pkg/enrollment`.
- [x] Added Rust agent `main.rs` and integrated test helpers.
- [x] Updated project documentation and architecture context.
- [x] Implemented robust workspace-root resolution logic in Rust tests (`test_helpers.rs`).
- [x] Refactored Rust agent test cleanup to use explicit async handling, preventing nested Tokio runtime panics.
- [x] Consolidated documentation into a WSL-first `README.md` and synchronized `memorybank`.

## Long-Term Direction

- Integrate full backup/storage workflow over the established mTLS channel.
- Implement server-side mDNS discovery for automated agent discovery.

## Important Reminders

- Do NOT implement logic before tests.
- Always verify working directory (project root for Docker commands, crate root for `cargo` commands) before running commands in WSL.
- Keep tests minimal and focused.
- Maintain separation of concerns and environments (WSL-first workflow).
- Remember the ephemeral nature of the CA; `ca.crt` must be re-extracted on server recreation.

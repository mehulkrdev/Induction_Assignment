# Active Context

## Current Focus

- Refactored and centralized runtime-scoped append-only logging system for both Rust and Go
- Fully implemented secure enrollment logic, including token validation and logging
- Expanding TDD to cover discovery and authenticated enrollment
- Maintaining strict environment separation (Windows vs WSL vs Docker)

## Current State

- Successful transport-layer connection between Windows Rust agent and Dockerized Go server
- Go server now implements POST /enroll with JSON payload parsing and validation
- All Go unit tests (POST /enroll validation) are PASSING in ephemeral containers
- All Rust integration tests (POST /enroll status handling) are PASSING against the running server
- Environment isolation is strictly maintained
- Logging system uses `DD-MM-YYYY_HH:MM` format and captures filename/line numbers

## Known Issues

- Need to ensure Go server is fully started before running Rust integration tests
- Managing Docker lifecycle (up/down) during the test-implement-refactor cycle

## Immediate Next Steps

1. Implement server-side mDNS discovery (Stage 1)
2. Implement BEB verification on the agent (Stage 2)
3. Expand `/enroll` to accept and validate HMAC-SHA256 tokens (Stage 3)
4. Ensure all new logic is preceded by failing tests

## Short-Term Goals

- [x] Agent connection test
- [x] Basic enrollment request test with token validation (RED Phase complete)
- [x] Basic enrollment request implementation (GREEN Phase complete)
- Server-side discovery (mDNS) implementation
- Agent-side server verification (BEB)

## Long-Term Direction

- Implement enrollment logic after tests
- Add TLS and certificate handling
- Transition to mTLS communication
- Integrate full backup/storage workflow

## Important Reminders

- Do NOT implement logic before tests
- Always verify working directory before running commands
- Keep tests minimal and focused
- Maintain separation of environments at all times

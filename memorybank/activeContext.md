# Active Context

## Current Focus

- Transitioning from minimal connection to secure enrollment logic
- Expanding TDD to cover discovery and authenticated enrollment
- Maintaining strict environment separation (Windows vs WSL vs Docker)

## Current State

- Successful transport-layer connection between Windows Rust agent and Dockerized Go server
- Go server implements a minimal `/enroll` endpoint on port 8443
- Rust agent integration test successfully validates connection and response from the server
- Go unit tests run in ephemeral containers (`docker compose run --rm`)
- Rust integration tests run against a persistent container (`docker compose up -d`)
- `Dockerfile` updated to support both integration and unit testing

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
- Basic enrollment request test with token validation
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
# Active Context

## Current Focus

- Establishing Test Driven Development (TDD) workflow
- Writing minimal test cases for agent-server interaction
- Fixing environment and path-related issues
- Ensuring correct execution boundaries (Windows vs WSL vs Docker)

## Current State

- Rust agent project created on Windows
- Cargo setup working with MSVC toolchain
- Docker successfully running inside WSL Debian
- Go server test environment working inside Docker
- Basic placeholder test executed inside Docker container
- Initial Rust test setup started

## Known Issues

- Path confusion between Windows and WSL environments
- Cline not correctly identifying working directories
- Need to enforce strict execution rules via memory-bank

## Immediate Next Steps

1. Use Cline to generate minimal test cases
2. Validate Rust agent test execution from correct directory
3. Ensure Go tests run only inside Docker
4. Define connection test (agent → localhost:8443)
5. Expand toward enrollment flow tests

## Short-Term Goals

- Agent connection test (failing)
- Basic enrollment request test
- Server-side handler test (mocked)
- Establish clear test structure on both sides

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
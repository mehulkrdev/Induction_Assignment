# Repository Structure and TDD Execution Strategy

This document outlines the current, simplified repository structure and the strategy for executing TDD tests for both the Rust agent and Go server components.

## Repository Layout

```
Assignment/
├── agent/                        # Rust agent project (Windows)
│   ├── Cargo.toml                # Rust project manifest and dependencies
│   └── src/                      # Rust source code and tests
│       ├── lib.rs                # Library code with core logic and tests
│       └── main.rs               # Main application entry point
│
├── server/                       # Go server project (WSL/Docker)
│   ├── Dockerfile                # Dockerfile for building the Go server image
│   ├── go.mod                    # Go module definition and dependencies
│   ├── main.go                   # Go server main application code
│   └── enrollment_test.go        # Go server test files
│
├── docker-compose.yml            # Docker Compose configuration for the Go server
├── bootstrap-bundle.json         # Shared trust bundle for enrollment
└── memorybank/                   # Project documentation and context
    ├── activeContext.md
    ├── productContext.md
    ├── projectbrief.md
    ├── repositoryStructure.md    # This document
    ├── systemPatterns.md
    └── techContext.md
```

## Docker Build Context and Test Execution Strategy (Go Server)

### **Docker Build Context:**

The `docker-compose.yml` file is configured such that the `server` service uses the `./server` directory as its build context. This means that when the Docker image for the server is built, all files within the `Assignment/server` directory are copied into the `/app` directory within the Docker container.

```yaml
services:
  server:
    build:
      context: ./server
      dockerfile: Dockerfile
```

### **Go Test Execution Strategy (Unit Tests):**

To adhere to the TDD principles and ensure an isolated, ephemeral testing environment for the Go server, unit tests are executed using `docker compose run`.

*   **Command**: `wsl docker compose run --rm server go test -v ./...`
    *   `wsl`: Executes the command within the Windows Subsystem for Linux environment.
    *   `docker compose run`: Runs a one-off command in a new container.
    *   `--rm`: Automatically removes the container after it exits.
    *   `server`: Specifies the service defined in `docker-compose.yml` to use.
    *   `go test -v ./...`: Runs unit tests inside the ephemeral container.
    *   **Context**: The `Dockerfile` copies all files into `/app` to support this execution.

This strategy ensures that:
*   Each test run happens in a clean, isolated environment.
*   No persistent server instance is running during testing.
*   All necessary Go module files (like `go.mod`) are available in the correct context within the container for `go test` to function correctly.

## Rust Agent Test Execution Strategy (Integration Tests)

Rust tests that depend on the Go server are treated as integration tests and require the server to be running.

### **Command Sequence:**

1.  `wsl docker compose up -d server`
2.  `wsl sleep 5` (Allow server to initialize)
3.  `cargo test --manifest-path agent/Cargo.toml`
4.  `wsl docker compose down`

*   `wsl docker compose up -d server`: Starts the Go server container in the background.
*   `cargo test`: Executes the Rust tests on Windows.
*   `wsl docker compose down`: Cleans up the environment after testing.

This strategy ensures that the Rust agent can communicate with a live Go server running in the Dockerized WSL environment.

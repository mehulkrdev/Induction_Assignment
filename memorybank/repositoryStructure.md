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

### **Go Test Execution Strategy:**

To adhere to the TDD principles and ensure an isolated, ephemeral testing environment for the Go server, tests are executed using `docker compose run`.

*   **Command**: `wsl docker compose run --rm server go test -v ./...`
    *   `wsl`: Executes the command within the Windows Subsystem for Linux environment.
    *   `docker compose run`: Runs a one-off command in a new container.
    *   `--rm`: Automatically removes the container after it exits.
    *   `server`: Specifies the service defined in `docker-compose.yml` to use.
    *   `go test -v ./...`: The command executed inside the container. This runs all tests in the current module (`.`) with verbose output (`-v`). The current working directory inside the container is `/app` due to `WORKDIR /app` in the `Dockerfile`.

This strategy ensures that:
*   Each test run happens in a clean, isolated environment.
*   No persistent server instance is running during testing.
*   All necessary Go module files (like `go.mod`) are available in the correct context within the container for `go test` to function correctly.

## Rust Agent Test Execution Strategy

### **Command:** `cargo test --manifest-path agent/Cargo.toml`

*   `cargo test`: Executes the tests defined in the Rust project.
*   `--manifest-path agent/Cargo.toml`: Specifies the path to the `Cargo.toml` file for the `agent` project, ensuring tests are run for the correct project on the Windows host.

This strategy ensures that Rust agent tests are executed directly on the Windows environment, as required, and targets the correct project.
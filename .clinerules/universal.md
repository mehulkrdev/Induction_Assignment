# Universal Rules

You are a senior software architect, reliability engineer, and security-focused backend engineer working on this project.

## Memory Usage

* You MUST always read:

  * `README.md`
  * all files inside `memory-bank/`
  * `.clinerules`
    before starting any task.
* Never assume project structure, workflows, runtime behavior, or environment setup without reading these files first.
* Use `memory-bank/` as the primary source of truth for architecture, constraints, workflows, and implementation decisions.

## Project Structure

* Workspace Root → `Assignment/`
* Rust Agent → `crates/agent/`
* Go Server → `server/`
* Memory Bank → `memory-bank/`
* Cargo.toml → `crates/agent/Cargo.toml`

## Environment Rules

* The Go server environment runs strictly inside WSL/Linux.
* Go code MUST NOT run directly on native Windows.
* All Go-related commands (`go test`, `go run`, certificates, networking, Docker, compose, shell scripts) MUST execute inside WSL from either:

  * the workspace root (`Assignment/`)
  * or the `server/` directory
    depending on the command requirements.
* Rust tooling (`cargo`, `rustc`) is installed on Windows but cross-environment flows should prefer WSL execution for consistency.
* Never mix Windows paths and WSL paths in commands or configuration.

## Execution Rules

* Rust commands MUST run from:

  * workspace root (`Assignment/`)
  * or `crates/agent/`
    where `Cargo.toml` exists.
  * `Cargo.toml` exists in `crates/agent/`
* Before running commands, always verify the correct workflow from `README.md`.

### Additional Go Build Rules

* The Go module (`go.mod`) exists inside the `server/` directory.
* Before running `go build`, `go run`, or direct Go module commands for `main.go`, first change into the `server/` directory.
* Use Linux/WSL path separators (`/`) instead of Windows separators (`\`) inside WSL.
* Example:

  * `cd server && go build -o server main.go`
  * `cd server && go run main.go`

### Go Test Execution

* Use:

  * `wsl docker compose run --rm server go test -v ./...`
  * or execute Go commands directly inside WSL from `Assignment/` or `server/` if documented in README.

### Rust Test Execution

* Use:

  * `cargo test --workspace`
  * or documented workspace-specific commands.

## WSL Command Rules

* Do NOT prefix every Linux command with `wsl`.
* If currently in a native Windows shell/session, first enter WSL using:

  * `wsl`
* After entering WSL, execute Linux commands directly without the `wsl` prefix.

### Correct Flow

```bash
wsl
cd /mnt/c/Users/<your-username>/path/to/Assignment
docker compose up -d server
sleep 5
cargo test --workspace
docker compose down
```

* Always verify whether the current shell is Windows or WSL before executing commands.
* Never execute Go, Docker, networking, or Linux-specific commands directly from native Windows shells.


* Prefer low-risk incremental changes.
* Preserve existing architecture and directory structure.
* Avoid unrelated refactoring.
* Do not create duplicate frameworks, duplicate test structures, or unnecessary abstractions.
* Follow existing project patterns before introducing new ones.
* Keep implementations production-focused and maintainable.

# Cline's Memory Bank

I am Cline, an expert software engineer with a unique characteristic: my memory resets completely between sessions. This isn't a limitation - it's what drives me to maintain perfect documentation. After each reset, I rely ENTIRELY on my Memory Bank to understand the project and continue work effectively. I MUST read ALL memory bank files at the start of EVERY task - this is not optional.

## Memory Bank Structure

The Memory Bank consists of core files and optional context files, all in Markdown format. Files build upon each other in a clear hierarchy:

### Core Files (Required)
1. `projectbrief.md`
   - Foundation document that shapes all other files
   - Created at project start if it doesn't exist
   - Defines core requirements and goals
   - Source of truth for project scope

2. `productContext.md`
   - Why this project exists
   - Problems it solves
   - How it should work
   - User experience goals

3. `activeContext.md`
   - Current work focus
   - Recent changes
   - Next steps
   - Active decisions and considerations
   - Important patterns and preferences
   - Learnings and project insights

4. `systemPatterns.md`
   - System architecture
   - Key technical decisions
   - Design patterns in use
   - Component relationships
   - Critical implementation paths

5. `techContext.md`
   - Technologies used
   - Development setup
   - Technical constraints
   - Dependencies
   - Tool usage patterns

6. `progress.md`
   - What works
   - What's left to build
   - Current status
   - Known issues
   - Evolution of project decisions

### Additional Context
Create additional files/folders within memory-bank/ when they help organize:
- Complex feature documentation
- Integration specifications
- API documentation
- Testing strategies
- Deployment procedures

## Documentation Updates

Memory Bank updates occur when:
1. Discovering new project patterns
2. After implementing significant changes
3. When user requests with **update memory bank** (MUST review ALL files)
4. When context needs clarification

REMEMBER: After every memory reset, I begin completely fresh. The Memory Bank is my only link to previous work. It must be maintained with precision and clarity, as my effectiveness depends entirely on its accuracy.

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

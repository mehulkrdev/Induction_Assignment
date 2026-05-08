1 | # Active Context
2 | 
3 | ## Current Focus
4 | 
5 | - Transitioning from minimal connection to secure enrollment logic
6 | - Expanding TDD to cover discovery and authenticated enrollment
7 | - Maintaining strict environment separation (Windows vs WSL vs Docker)
8 | 
9 | ## Current State
10 | 
11 | - Successful transport-layer connection between Windows Rust agent and Dockerized Go server
12 | - Go server now expects POST /enroll with JSON payload; tests are failing as expected
13 | - Rust agent integration tests updated to send POST /enroll with JSON payload; tests are failing as expected
14 | - Go unit tests run in ephemeral containers (`docker compose run --rm`)
15 | - Rust integration tests run against a persistent container (`docker compose up -d`)
16 | - `Dockerfile` updated to support both integration and unit testing
17 | 
18 | ## Known Issues
19 | 
20 | - Need to ensure Go server is fully started before running Rust integration tests
21 | - Managing Docker lifecycle (up/down) during the test-implement-refactor cycle
22 | 
23 | ## Immediate Next Steps
24 | 
25 | 1. Implement server-side mDNS discovery (Stage 1)
26 | 2. Implement BEB verification on the agent (Stage 2)
27 | 3. Expand `/enroll` to accept and validate HMAC-SHA256 tokens (Stage 3)
28 | 4. Ensure all new logic is preceded by failing tests
29 | 
30 | ## Short-Term Goals
31 | 
32 | - [x] Agent connection test
33 | - [x] Basic enrollment request test with token validation (RED Phase complete for Go and Rust)
34 | - Server-side discovery (mDNS) implementation
35 | - Agent-side server verification (BEB)
36 | 
37 | ## Long-Term Direction
38 | 
39 | - Implement enrollment logic after tests
40 | - Add TLS and certificate handling
41 | - Transition to mTLS communication
42 | - Integrate full backup/storage workflow
43 | 
44 | ## Important Reminders
45 | 
46 | - Do NOT implement logic before tests
47 | - Always verify working directory before running commands
48 | - Keep tests minimal and focused
49 | - Maintain separation of environments at all times
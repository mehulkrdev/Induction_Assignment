# Project Run & Verification Guide

This guide provides step-by-step instructions to run the server and agent, and to verify that the mutual TLS (mTLS) connection is successful.

---

## Prerequisites

*   **Docker & Docker Compose**: Installed and running (via WSL2 recommended).
*   **Rust**: Installed (for running the agent tests).
*   **OpenSSL**: For manual key/certificate operations (optional).

---

## Step 1 — Start the Go Server

The server is dockerized and manages its own Certificate Authority (CA).

1.  Navigate to the project root:
    ```bash
    cd Assignment
    ```
2.  Build the server Docker image and start the server using Docker Compose:
    ```bash
    wsl docker compose build server && wsl docker compose up -d server --force-recreate
    ```
3.  Verify the server is running:
    ```bash
    wsl docker ps
    ```
    *You should see `assignment-server-1` running and exposing ports `8443` (Enrollment) and `8444` (mTLS).*

---

## Step 2 — Extract the CA Certificate

The agent needs the server's CA certificate to verify the server's identity.

1.  **From the project root**, copy the `ca.crt` from the running container:
    ```bash
    docker cp assignment-server-1:/app/ca.crt .
    ```

---

## Step 3 — Run the Rust Agent (Automated Verification)

We use the built-in integration tests to verify the complete flow: Enrollment -> Certificate Persistence -> mTLS Reconnection.

1.  Navigate to the agent directory:
    ```bash
    cd crates/agent
    ```
2.  Ensure `ca.crt` is available in this directory (it should be if you ran the `docker cp` command correctly in Step 2, as the root directory is a parent):
    *Note: If the test fails with "ca.crt not found", copy it into this folder:*
    ```bash
    cp ../../ca.crt .
    ```
3.  Run the integration tests:
    ```bash
    cargo test -p enrollment-agent --test enrollment_test -- --nocapture
    ```

### How to identify success:
*   **Test Result**: Look for `running 5 tests` followed by `test ... ok` for all tests.
*   **Log Output**: The agent logs its progress. Look for:
    *   `Starting enrollment for agent: agent-test`
    *   `Enrollment successful. Certificate and key saved.`
    *   `Attempting mTLS reconnection for agent: agent-test`
    *   `mTLS reconnection successful: Hello verified agent: agent-test`
*   **New Error Handling Tests**: Verify that the new tests for 401, 409, and malformed JSON responses pass with appropriate error messages.

---

## Step 4 — Manual Verification (Using Curl)

If you prefer to verify the connection manually, follow these steps from the **project root**:

1.  Navigate back to the project root:
    ```bash
    cd ..\
    ```

### A. Generate Agent Keypair
```bash
openssl ecparam -name prime256v1 -genkey -noout -out manual_agent.key
openssl ec -in manual_agent.key -pubout -out manual_agent.pub
```

### B. Prepare Public Key for JSON
```bash
# This formats the public key for inclusion in a JSON string
PUB_KEY=$(awk '{printf "%s\\n", $0}' manual_agent.pub)
```

### C. Enroll Agent via Curl
```bash
	curl -vk https://localhost:8443/enroll \
  -H "Content-Type: application/json" \
  -d "{
    \"agent_id\":\"manual-agent\",
    \"enrollment_token\":\"valid-token\",
    \"public_key\":\"$PUB_KEY\"
  }"
```
*Expected: HTTP 200 with a JSON containing the signed certificate.*
*Expected Error Case (Duplicate): Subsequent enrollments with same agent_id should return 409 Conflict.*
*Expected Error Case (Invalid Token): Enrollments with an invalid_token should return 401 Unauthorized.*

### D. Save the Certificate
Extract the `certificate` field from the JSON response and save it as `manual_agent.crt`. Ensure newlines are preserved.

### E. Verify mTLS Connection
```bash
curl -vk https://localhost:8444/secure \
  --cert manual_agent.crt \
  --key manual_agent.key \
  --cacert ca.crt
```

### How to identify success:
*   **HTTP Status**: Look for `HTTP/2 200`.
*   **Response Body**: Look for `Hello verified agent: manual-agent`.
*   **TLS Handshake**: In the verbose output (`-v`), you should see:
    *   `* TLSv1.3 (OUT), TLS handshake, Client hello (1):`
    *   `* TLSv1.3 (IN), TLS handshake, Server hello (2):`
    *   `* TLSv1.3 (IN), TLS handshake, Request CERT (13):` (Server asking for client cert)
    *   `* TLSv1.3 (OUT), TLS handshake, Certificate (11):` (Client sending cert)

---

## Troubleshooting

*   **Go Server Build Failures**: If `wsl docker compose build server` fails, check `server/main.go`, `server/pkg/enrollment/service.go`, and `server/tests/enrollment_test.go` for compilation errors related to import paths or syntax. Ensure all changes are correctly applied.
*   **Go Server Test Failures**: If `wsl docker compose run --rm server go test -v ./...` fails, review the specific test outputs for assertions. Ensure the in-memory `enrolledAgents` map is reset or handled appropriately between test runs if state is shared.
*   **Connection Refused**: Ensure Docker containers are running (`wsl docker ps`).
*   **CA Certificate Mismatch**: If you rebuild or restart the server with a clean volume, you MUST re-run the `wsl docker cp assignment-server-1:/app/ca.crt .` command to get the new `ca.crt`.
*   **Port Conflicts**: Ensure ports `8443` and `8444` are not being used by other applications.




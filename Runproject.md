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

The agent and manual `curl` commands need the server's CA certificate to verify the server's identity.

1.  **From the project root**, create the `data/` directory if it doesn't exist:
    ```bash
    mkdir -p data
    ```
2.  Copy the `ca.crt` from the running `assignment-server-1` container to the `data/` directory on your host:
    ```bash
    wsl docker cp assignment-server-1:/app/data/ca.crt ./data/ca.crt
    ```

---

## Step 3 — Run the Rust Agent (Automated Verification)

We use the built-in integration tests to verify the complete flow: Enrollment -> Certificate Persistence -> mTLS Reconnection.

1.  Navigate to the agent directory:
    ```bash
    cd crates/agent
    ```
2.  The agent tests will automatically locate the `ca.crt` from the `data/` directory at the project root.
3.  Run the integration tests:
    ```bash
    cargo test -p enrollment-agent --test enrollment_test -- --nocapture
    ```

### How to identify success:
*   **Test Result**: Look for `running 6 tests` followed by `test ... ok` for all tests.
*   **Log Output**: The agent logs its progress. Look for:
    *   `Starting enrollment for agent: agent-test`
    *   `Enrollment successful. Certificate and key saved.`
    *   `Attempting mTLS reconnection for agent: agent-test`
    *   `mTLS reconnection successful: Hello verified agent: agent-test`
*   **New Error Handling Tests**: Verify that the new tests for 401, 409, and malformed JSON responses pass with appropriate error messages.

---

## Step 4 — Manual Verification (Using Curl)

Manual verification allows you to test the enrollment and mTLS flow without the Rust agent. **Ensure the Go server is running and `ca.crt` has been extracted to `data/ca.crt` (see Steps 1 & 2) before proceeding.**

Follow these steps from the **project root**:

### A. Generate Agent Keypair
Generate a new ECDSA P-256 keypair for the manual agent:
```bash
openssl ecparam -name prime256v1 -genkey -noout -out manual_agent.key
openssl ec -in manual_agent.key -pubout -out manual_agent.pub
```

### B. Prepare Public Key for JSON
The public key must be properly formatted (newlines escaped) to be sent in a JSON payload:
```bash
# Run this in WSL/Linux to format the key into a variable
PUB_KEY=$(awk '{printf "%s\\n", $0}' manual_agent.pub)
```

### C. Enroll Agent via Curl
Send the enrollment request and automatically save the received certificate to `manual_agent.crt` in the project root.

Run the following command in WSL/Linux:
```bash
# This command sends the enrollment request, parses the JSON response using 'jq', 
# handles escaped newlines, and saves the certificate to manual_agent.crt.
curl -sk https://localhost:8443/enroll \
  -H "Content-Type: application/json" \
  -d "{
    \"agent_id\":\"manual-agent\",
    \"enrollment_token\":\"valid-token\",
    \"public_key\":\"$PUB_KEY\"
  }" | jq -r .certificate | sed 's/\\n/\n/g' > manual_agent.crt
```
*   **Note**: This requires `jq` to be installed (`sudo apt install jq`).
*   **Verification**: Ensure `manual_agent.crt` exists and contains a valid certificate.
*   **Expected Error Case (Duplicate)**: If the agent is already enrolled, the command might fail or save an empty file. Check the server response.

### D. Verify mTLS Connection
Test the mutual TLS connection by accessing the secure endpoint on port `8444`:
```bash
curl -vk https://localhost:8444/secure \
  --cert manual_agent.crt \
  --key manual_agent.key \
  --cacert data/ca.crt
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
*   **CA Certificate Mismatch**: If you rebuild or restart the server with a clean volume, you MUST re-run the `wsl docker cp assignment-server-1:/app/data/ca.crt ./data` command to get the new `ca.crt`.
*   **Port Conflicts**: Ensure ports `8443` and `8444` are not being used by other applications.




# Secure Enrollment & mTLS Communication Platform

## Description

This project implements a secure cross-platform enrollment and mutual TLS (mTLS) communication system using:

* A Rust-based Windows Agent
* A Go-based Debian Server running inside Docker on WSL

The platform enables secure device enrollment using certificate-based authentication.
The agent dynamically generates its own cryptographic identity, enrolls securely with the server over HTTPS, receives a signed client certificate, and later establishes a fully authenticated mTLS connection.

The implementation follows practical Test-Driven Development (TDD) principles, clean repository structuring, Dockerized deployment, and modern TLS security practices.

---

# Problem Statement

The goal of this assignment was to design and implement a secure enrollment mechanism between:

* A Rust-based client agent running on Windows
* A Go-based backend server running on Debian (inside WSL/Docker)

The system needed to:

* Establish secure communication
* Generate and manage cryptographic identities
* Perform certificate-based enrollment
* Support mutual TLS authentication
* Follow TDD-oriented development practices
* Use Docker for isolated deployment
* Maintain proper project organization and testing structure

---

# Architecture Overview

## Components

### Rust Agent

Responsible for:

* Generating ECDSA P-256 keypairs
* Sending enrollment requests
* Persisting issued certificates
* Establishing mTLS communication

### Go Server

Responsible for:

* Generating a local Certificate Authority (CA)
* Generating server certificates
* Signing agent certificates
* Hosting HTTPS and mTLS endpoints

### Docker + WSL

Responsible for:

* Debian-based isolated deployment
* Consistent runtime environment
* Port exposure and networking

---

# Features Implemented

## Secure Enrollment

* HTTPS-based enrollment endpoint (`8443`)
* Enrollment token validation
* Public key submission from agent
* Server-side certificate signing

## Mutual TLS (mTLS)

* Client certificate authentication
* CA-based trust validation
* Bidirectional identity verification
* Secure communication over port `8444`

## Certificate Infrastructure

* Self-signed local Certificate Authority
* Dynamic server certificate generation
* Dynamic client certificate generation
* PEM encoding/decoding support

## Persistent Identity

The agent stores:

* Private key (`agent.key`)
* Public key (`agent.pub`)
* Signed certificate (`agent.crt`)

## Dockerized Deployment

* Go server runs inside Docker
* Debian-based runtime environment
* Exposed TLS ports for enrollment and mTLS

## TDD-Oriented Structure

* Integration tests separated from implementation
* Dedicated `tests/` directories
* Mock-based behavioral testing

---

# Technologies Used

## Rust

* tokio
* reqwest
* rustls
* p256
* pkcs8
* serde

## Go

* crypto/tls
* crypto/x509
* net/http
* encoding/pem

## DevOps / Environment

* Docker
* Docker Compose
* WSL (Windows Subsystem for Linux)

---

# Project Directory Structure

```text
Assignment/
│
├── agent/
│   ├── src/
│   │   └── lib.rs
│   │
│   ├── tests/
│   │   └── enrollment_test.rs
│   │
│   └── Cargo.toml
│
├── server/
│   ├── main.go
│   ├── Dockerfile
│   │
│   ├── pkg/
│   │   ├── certutil/
│   │   └── logger/
│   │
│   ├── tests/
│   │   └── enrollment_test.go
│   │
│   └── go.mod
│
├── logger/
│   ├── client/
│   └── server/
│
├── memorybank/
│
├── docker-compose.yml
│
└── README.md
```

---

# Enrollment + mTLS Flow

## Step 1 — Server Bootstrap

The Go server:

* Generates a local CA
* Generates server certificates
* Starts HTTPS server on `8443`
* Starts mTLS server on `8444`

## Step 2 — Agent Key Generation

The Rust agent:

* Generates ECDSA P-256 keypair
* Stores:

  * `agent.key`
  * `agent.pub`

## Step 3 — Enrollment

The agent:

* Sends public key to `/enroll`
* Includes enrollment token
* Receives signed certificate from server

## Step 4 — Certificate Persistence

The agent stores:

* `agent.crt`

## Step 5 — mTLS Communication

The agent:

* Presents certificate to server
* Proves ownership of private key
* Establishes mutual TLS connection

The server:

* Verifies client certificate against CA
* Authenticates agent identity
* Grants access to secure endpoint

---

# How the Assignment Was Handled

The implementation was developed incrementally using a TDD-oriented workflow.

## Initial Phase

* Designed project structure
* Set up Rust and Go services
* Created Dockerized Go environment

## Security Implementation

* Implemented CA generation
* Added HTTPS support
* Added client certificate signing
* Added mTLS endpoint

## Refactoring

* Removed dead code
* Removed unused discovery components
* Moved tests into dedicated `tests/` directories
* Improved certificate encapsulation

## Validation

The complete system was manually verified by:

* Enrolling an agent
* Receiving signed certificates
* Establishing mTLS communication
* Verifying certificate-based authentication

---

# Test Strategy (TDD)

## Rust Tests

Located in:

```text
agent/tests/
```

Tests include:

* Enrollment flow validation
* Certificate persistence
* mTLS reconnection behavior

## Go Tests

Located in:

```text
server/tests/
```

Tests include:

* Enrollment endpoint validation
* Token validation
* Certificate signing logic

---

# Setup & Execution Steps

## Step 1 — Open WSL

Navigate to the cloned project directory:

cd <path-to-project>/Assignment

Example:

cd ~/Assignment

or

cd /mnt/c/Users/<your-username>/Downloads/Assignment

---

## Step 2 — Start Docker Server

```bash
docker compose up --build -d
```

Verify container:

```bash
docker ps
```

---

## Step 3 — Verify HTTPS Enrollment Endpoint

```bash
curl -vk https://localhost:8443/enroll
```

Expected:

```text
HTTP/2 405
Method not allowed
```

---

## Step 4 — Generate Agent Keypair

```bash
openssl ecparam -name prime256v1 -genkey -noout -out agent.key

openssl ec -in agent.key -pubout -out agent.pub
```

---

## Step 5 — Prepare Public Key Variable

```bash
PUB_KEY=$(awk '{printf "%s\\n", $0}' agent.pub)
```

---

## Step 6 — Enroll Agent

```bash
curl -vk https://localhost:8443/enroll \
  -H "Content-Type: application/json" \
  -d "{
    \"agent_id\":\"agent-001\",
    \"enrollment_token\":\"valid-token\",
    \"public_key\":\"$PUB_KEY\"
  }"
```

Expected:

* HTTP 200
* Signed certificate returned

---

## Step 7 — Save Returned Certificate

Save response certificate as:

```text
agent.crt
```

---

## Step 8 — Extract CA Certificate

```bash
docker cp assignment-server-1:/app/ca.crt .
```

---

## Step 9 — Verify mTLS Connection

```bash
curl -vk https://localhost:8444/secure \
  --cert agent.crt \
  --key agent.key \
  --cacert ca.crt
```

Expected:

```text
HTTP/2 200
```

This confirms:

* Mutual TLS authentication
* Certificate validation
* Secure identity verification

---
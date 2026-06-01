# Project Brief

## Overview
This project implements a secure enrollment system between a Windows-based Rust agent and a Dockerized Go server running in a WSL environment. The system establishes trust and transitions from zero-knowledge to fully authenticated mutual TLS (mTLS) communication.

## Objective
To establish a secure, trusted connection between agent and server starting from a shared root CA and progressing to mutual TLS (mTLS) for all subsequent communications.

## Core Enrollment Flow

1. **Discovery & Trust**: Agent starts with a trusted `ca.crt` (Bootstrap trust).
2. **Key Generation**: Agent generates an ECDSA P-256 keypair.
3. **Enrollment Request**: Agent sends a POST request to `https://localhost:8443/enroll` with:
    - Enrollment Token (HMAC-SHA256)
    - Agent ID
    - Public Key (PEM)
4. **Validation & Signing**: Server validates the token, signs the agent's public key using its internal CA, and returns a client certificate.
5. **Persistence**: Agent saves the issued certificate and private key.
6. **Secure Communication**: Agent performs an mTLS handshake on `https://localhost:8444/secure` to verify the connection.

## Key Constraints

- **Strict Test Driven Development (TDD)**: No implementation before tests.
- **Environment Isolation**: Server runs inside Docker (WSL); Agent runs on Windows.
- **Security**: ECDSA P-256 for keys, HTTPS for enrollment, mTLS for communication.
- **Logging**: Standardized centralized logging for auditability.

## Expected Outcome

- A fully automated enrollment flow.
- Verified secure communication via mTLS.
- Robust test coverage for both agent and server components.
- Portable and clean repository structure.

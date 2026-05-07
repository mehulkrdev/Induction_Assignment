# Project Brief

## Overview
This project implements a secure enrollment system between a Windows-based Rust agent and a Debian-based Go server running inside Docker in WSL.

## Objective
To establish a secure, trusted connection between agent and server starting from zero trust and progressing to mutual TLS (mTLS).

## Core Enrollment Flow

1. Agent discovers server using mDNS
2. Agent verifies server identity using BEB (Bootstrap Enrollment Bundle with CA fingerprint)
3. Agent connects to server on port 8443
4. Agent sends enrollment request using HMAC-SHA256 token
5. Server validates token and issues client certificate
6. Agent uses issued certificate for mTLS communication on port 8444

## Key Constraints

- Strict Test Driven Development (TDD)
- No implementation before tests
- Minimal, focused test cases only
- Server must run inside Docker (WSL Debian)
- Agent runs on Windows
- Clear separation between environments

## Expected Outcome

- Defined test cases for agent-server interaction
- Verified connection attempt on localhost:8443
- Foundation for secure enrollment and communication
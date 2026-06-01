# Product Context

## Why this project exists
This project provides a robust foundation for secure agent-server communication in zero-trust environments. It ensures that any agent attempting to connect must first be explicitly authorized and issued a unique identity before accessing secure services.

## Problems it solves

- **Trust Establishment**: Securely establishing trust between a Windows agent and a Linux-based server appliance.
- **Secure Enrollment**: Preventing unauthorized enrollment through token-based validation.
- **Identity Management**: Automating the generation, signing, and persistence of ECDSA P-256 identities.
- **Encrypted Communication**: Ensuring all operational traffic is protected by mutual TLS (mTLS).

## How it works

1. **Bootstrap Trust**: The agent uses a pre-shared or discovered CA certificate to verify the server.
2. **Automated Enrollment**: The agent generates a keypair and requests a certificate using a one-time enrollment token.
3. **Certificate Issuance**: The server validates the request and issues a short-lived or permanent client certificate.
4. **mTLS Handshake**: Subsequent communication transitions to a dedicated secure port (8444) where both parties must prove their identity.

## User Experience Goals

- **Zero Manual Configuration**: Enrollment should be transparent and automatic once a token is provided.
- **Secure by Default**: No unencrypted or unauthenticated channels are used for sensitive operations.
- **Clear Auditability**: All enrollment and connection events are logged in a standardized format.
- **Environment Agnostic**: The system works seamlessly across Windows host and Dockerized Linux environments.

## Key Principles

- **Security First**: No shortcuts in trust establishment or certificate handling.
- **TDD-Driven**: Every feature is defined by a test before it is implemented.
- **Minimalism**: Focus on the simplest, most secure solution that fulfills the requirements.
- **Separation of Concerns**: Enrollment logic is kept distinct from operational communication.

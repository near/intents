# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build System

This project uses `cargo-make` for build orchestration:

- **Build all contracts**: `cargo make build`
- **Run tests**: `cargo make test`
- **Run clippy linter**: `cargo make clippy`
- **Clean build artifacts**: `cargo make clean`

Built contracts are placed in the `res/` directory after building.

## Project Architecture

This is a Rust workspace containing NEAR blockchain smart contracts for the NEAR Intents system (formerly "defuse"). The main smart contract is the "Verifier" which facilitates atomic P2P transactions.

### Core Components

- **defuse/**: Main smart contract (the "Verifier") that executes intents and manages accounts/tokens
- **core/**: Core types and logic including intents system, engine, accounts, amounts, and payload handling
- **crypto/**: Cryptographic utilities supporting multiple curves (ed25519, p256, secp256k1)
- **tests/**: Integration tests using near-workspaces

### Key Modules

- **Intent System**: Located in `core/src/intents/` - defines various intent types (Transfer, FtWithdraw, NftWithdraw, etc.) and execution engine
- **Account Management**: `defuse/src/accounts/` handles user accounts, authentication, and key management
- **Token Support**: Multiple token standards (NEP-141 FT, NEP-171 NFT, NEP-245 MT) in respective modules
- **Payload Handling**: `core/src/payload/` supports various signature schemes (BIP322, ERC191, NEP413, etc.)

### Token Standard Libraries

The project includes several NEP (NEAR Enhancement Proposal) implementations:
- **nep245/**: Multi-token standard implementation
- **nep413/**: Message signing standard
- **nep461/**: Multi-token events

### Supporting Contracts

- **poa-factory/** and **poa-token/**: Proof of Authority bridge contracts for cross-chain token transfers
- **controller/**: Contract upgrade interface following Aurora controller pattern

### Utility Crates

- **near-utils/**: NEAR-specific utilities (gas, time, locks, etc.)
- **crypto/**, **serde-utils/**, **borsh-utils/**: General-purpose utilities
- **bip340/**: BIP-340 cryptographic primitives (double hash, tagged hash) with digest trait compatibility
- **bip322/**: Bitcoin BIP-322 message signing implementation using NEAR SDK and BIP340 integration
- **test-utils/**: Testing helpers and assertions

## Development Notes

- Rust edition 2024, minimum version 1.86.0
- Strict clippy lints enabled (all, pedantic, nursery levels set to deny)
- Uses NEAR SDK 5.15 and near-plugins for access control and pausability
- Main contract implements role-based access control with roles like DAO, FeesManager, etc.

## Testing

Integration tests are comprehensive and located in `tests/src/tests/`. They test the full contract functionality including:
- Intent execution and token transfers  
- Account management and authentication
- Token deposits/withdrawals for all supported standards
- Multi-token operations and storage management
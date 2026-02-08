# CLAUDE.md - AI Assistant Guide for NEAR Intents

## Project Overview

NEAR Intents is a smart contract ecosystem for the NEAR
Protocol that facilitates atomic peer-to-peer transactions.
The main contract ("Verifier" / "defuse") allows trustless
token swaps by evaluating signed intents and executing them
atomically.

The repository also includes:
- **escrow-swap** - Deterministic escrow contract for
  token swaps
- **poa-factory** / **poa-token** - Proof of Authority
  bridge contracts for cross-chain token transfers
  (Bitcoin, Ethereum, Solana, etc.)

## Repository Structure

This is a **Rust workspace** with 33 crates.

### Smart Contracts
- `defuse/` - Main Verifier contract (v0.4.1) for
  executing intents atomically
- `escrow-swap/` - Deterministic escrow contract for
  token swaps
- `poa-factory/` - POA bridge factory contract
- `poa-token/` - POA bridge token contract

### Core Libraries
- `core/` - Intent engine, payload processing, state
  management (`engine/`, `payload/`, `intents/`)
- `crypto/` - Cryptographic operations (ed25519,
  ECDSA/P-256), payload traits

### Signing Standards
Each standard has its own crate implementing the
`Payload` / `SignedPayload` traits:
- `erc191/` - Ethereum ERC-191
- `nep413/` - NEAR NEP-413
- `nep461/` - NEAR NEP-461
- `sep53/` - Stellar SEP-53
- `tip191/` - Tron TIP-191
- `ton-connect/` - TON Connect
- `webauthn/` - WebAuthn (passkeys)

### Token Standards
- `nep245/` - Multi-token standard
- `token-id/` - Token identifier types

### Utilities
- `admin-utils/` - Admin/access control helpers
- `auth-call/` - Authenticated cross-contract call
  support
- `bitmap/` - Bitmap data structure
- `borsh-utils/`, `serde-utils/` - Serialization helpers
- `controller/` - Aurora controller interface
- `deadline/` - Deadline/expiration handling
- `decimal/`, `num-utils/` - Numeric utilities
- `fees/` - Fee calculation
- `io-utils/` - I/O helpers
- `map-utils/` - Map/collection utilities
- `near-utils/` - NEAR-specific utilities
- `wnear/` - Wrapped NEAR

### Testing
- `tests/` - Integration test suite (near-sandbox based)
- `test-utils/` - Shared test utilities
- `sandbox/` - NEAR sandbox setup helpers
- `randomness/` - Test randomness support

### Build Artifacts
- `res/` - Compiled WASM contracts (generated,
  gitignored except checksums)

## Build System

**Toolchain**: Rust 1.86.0, Edition 2024,
target `wasm32-unknown-unknown`

**Required tools**: `cargo-make`, `cargo-near` (v0.17.0)

### Essential Commands

```shell
# Build all contracts (outputs to res/)
cargo make build

# Run integration tests (builds first, then tests)
cargo make test

# Run linter (strict clippy)
cargo make clippy

# Run tests with long-running features enabled
cargo make run-tests-long

# Format check
cargo fmt --check

# Clean build artifacts
cargo make clean

# Reproducible WASM builds (requires Docker)
cargo make build-reproducible
```

### Build Details

- `cargo make build` builds: defuse, escrow-swap,
  poa-factory, poa-token (no-registration variant),
  and multi-token-receiver-stub
- Contracts are compiled using
  `cargo near build non-reproducible-wasm`
- Feature flags used during build:
  - `abi,contract,imt` (defuse)
  - `abi,contract` (escrow-swap)
  - `contract` (poa-factory/poa-token)
- WASM artifacts are placed in `res/`

### Testing Details

- Integration tests use `near-sandbox` for local
  NEAR blockchain simulation
- Tests depend on pre-built WASM artifacts in `res/`
  (the `test` task builds first)
- For state migration testing:
  set `DEFUSE_MIGRATE_FROM_LEGACY=1`
- CI runs: `cargo make run-tests-long -- --show-output`
  with `DEFUSE_MIGRATE_FROM_LEGACY=true`

## CI Pipeline

The CI pipeline (`.github/workflows/ci.yml`) runs:
1. **Format** - `rustfmt` check
2. **Check** - `cargo make clippy` (strict linting)
3. **Build** - `cargo make build` (WASM compilation)
4. **Build Reproducible** - Docker-based reproducible
   WASM (parallel with Build)
5. **Tests** - `cargo make run-tests-long` (depends on
   Build artifacts)
6. **Security Audit** - `cargo audit` for
   vulnerabilities (report + deny modes)
7. **Contract Analysis** - Shared Aurora security
   analysis

## Code Conventions

### Clippy Configuration (Strict)

The workspace enforces very strict linting
(`Cargo.toml`):
```toml
[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "deny", priority = -1 }
nursery = { level = "deny", priority = -1 }
as_conversions = { level = "deny", priority = -1 }
```

Allowed exceptions: `module_name_repetitions`,
`missing_errors_doc`, `missing_panics_doc`,
`must_use_candidate`, `unreadable_literal`,
`similar_names`, `too_long_first_doc_paragraph`.

**All new code must pass these strict clippy rules.**
Use `#[allow(...)]` sparingly and only with
justification.

### Rust Conventions

- **Edition 2024** with resolver v3
- **Feature flags** are used extensively for
  conditional compilation:
  - `contract` - Enables smart contract entry points
    (NEAR `#[near]` macros)
  - `abi` - Enables ABI generation for documentation
  - `imt` - Indexed Merkle Tree support
  - `arbitrary` - Test data generation
  - Token standards: `nep141`, `nep171`, `nep245`
- **Trait-based architecture**: Core behaviors defined
  as traits (e.g., `Intents`, `AccountManager`),
  implementations in `contract/` submodules
- **Error handling**: Custom error types via
  `thiserror`, centralized `Result<T>` types
- **No `as` conversions**: The `as_conversions` lint
  is denied; use `.into()`, `.try_into()`, or explicit
  conversion functions
- **Overflow checks enabled** in release profile
  (`overflow-checks = true`)

### Project Naming

- Crate directories use short names: `core/`,
  `crypto/`, `bitmap/`
- Published crate names are prefixed: `defuse-core`,
  `defuse-crypto`, `defuse-bitmap`
- The name "defuse" is the legacy name for the
  Verifier contract, being phased out in favor of
  "NEAR Intents"

### Architecture Patterns

- **State versioning**: Contract state uses versioned
  enums for storage migration support (e.g.,
  `AccountEntryV0`, `AccountEntryV1`)
- **Nonce system**: Supports legacy nonces and
  versioned nonces with salts and deadlines
- **Multi-payload support**: The engine processes
  signed payloads from multiple signing standards
  uniformly through the `Payload` / `SignedPayload`
  traits
- **Modular token support**: Token operations
  separated by standard (NEP-141 fungible, NEP-171
  NFT, NEP-245 multi-token)

### Testing Patterns

- Unit tests colocated in source files
- Integration tests in `tests/` crate using
  `near-sandbox`
- Property-based testing with `proptest`
- Parameterized tests with `rstest`
- Arbitrary data generation for fuzzing

## Key Dependencies

| Dependency | Version | Purpose |
|---|---|---|
| `near-sdk` | 5.24 | NEAR contract framework |
| `near-contract-standards` | 5.24 | Standard traits |
| `near-plugins` | 0.5.0 | AccessControl, Pausable |
| `near-workspaces` | 0.22 | Integration tests |
| `cargo-near` | 0.17.0 | NEAR WASM compiler |
| `ed25519-dalek` | 2.1 | EdDSA signatures |
| `p256` | 0.13.2 | ECDSA with P-256 |
| `thiserror` | 2 | Error derive macros |
| `serde_with` | 3.9 | Advanced serialization |

## Common Tasks for AI Assistants

### Adding a new intent type
1. Define the intent structure in
   `core/src/intents/`
2. Implement processing logic in
   `core/src/engine/`
3. Wire it into the contract in
   `defuse/src/contract/intents/`
4. Add integration tests in
   `tests/src/defuse/intents/`

### Adding a new signing standard
1. Create a new crate in the workspace root
2. Implement `Payload` and `SignedPayload` traits
   from `defuse-crypto`
3. Register in `core/src/payload/` for multi-payload
   support
4. Add the crate to workspace members in root
   `Cargo.toml`

### Modifying contract state
- Always consider state migration: add a new version
  variant, implement migration logic
- Test with `DEFUSE_MIGRATE_FROM_LEGACY=1` to verify
  migration paths
- State is versioned for forward compatibility

### Before submitting changes
1. `cargo fmt` - Format code
2. `cargo make clippy` - Must pass strict linting
   (deny all + pedantic + nursery)
3. `cargo make test` - Build and run all tests
4. Check that no `as` casts are used (denied by lint
   rules)

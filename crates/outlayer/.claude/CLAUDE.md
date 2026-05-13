# Outlayer

Framework for **off-chain, app-specific wasm components** that extend Intents without touching on-chain contracts. Apps publish a wasmtime [Component Model](https://component-model.bytecodealliance.org/) binary; the service fetches it, verifies SHA-256 against an on-chain reference, compiles, runs in a sandbox.

Load-bearing piece of the architecture for adding programmable behavior (custom signing, validation, routing) without bloating the Verifier.

## On-chain ↔ off-chain split

```
on-chain:  defuse-outlayer-app          { admin_id, code_hash, code_url }
                                        AccountId = NEP-616(StateInit)
                       │
                       ▼
off-chain: Outlayer service →  Resolver (URL → bytes; verify SHA-256)
                               Cache (moka, keyed by hash)
                               Executor → vm-runner (wasmtime) + host imports
```

On-chain contract: `contracts/outlayer-app/.claude/CLAUDE.md`.

## Crates

| Crate                        | Path          | Side  | Role                                                  |
| ---------------------------- | ------------- | ----- | ----------------------------------------------------- |
| `defuse-outlayer-service`    | `service/`    | host  | Entry point: resolver + cache + executor.             |
| `defuse-outlayer-executor`   | `executor/`   | host  | Runs components with bounded stdio + fuel.            |
| `defuse-outlayer-vm-runner`  | `vm-runner/`  | host  | wasmtime runtime; CLI at `vm-runner/cli/`.            |
| `defuse-outlayer-host`       | `host/`       | host  | Impl of WIT imports (signer, host context).           |
| `defuse-outlayer-crypto`     | `crypto/`     | host  | Crypto for `host`.                                    |
| `defuse-outlayer-sdk`        | `sdk/`        | guest | Ergonomic wrapper over `sys`.                         |
| `defuse-outlayer-sys`        | `sys/`        | guest | Generated bindings from WIT.                          |
| `defuse-outlayer-primitives` | `primitives/` | both  | Shared types (`AppId`, …).                            |
| `wit/`                       | `wit/`        | both  | WIT defs (`outlayer:host`, `outlayer:crypto`).        |

Editing `wit/world.wit` or `wit/deps/*.wit` → regenerate `sys` (guest), update `host` impl, re-check `sdk/examples/signer.rs`.

## Entry points

**Host:**

```rust
use defuse_outlayer_service::{Outlayer, Code};
let outlayer = Outlayer::builder()./* … */.build()?;
let outcome = outlayer.execute(Code::Ref { code_ref }, input, fuel).await?;
// Code::Inline { code } skips on-chain lookup (tests / ad-hoc).
```

`Outcome { output, logs, execution }` — output = stdout (≤4 MB), logs = stderr (≤16 KB), execution = wasmtime status + fuel.

**Guest:** see `sdk/examples/signer.rs`. Pattern: read input from `io::stdin()`, use `defuse_outlayer_sdk::host::crypto::{ed25519, secp256k1}`, write result to `io::stdout()`.

## WIT host interface (`outlayer:crypto`)

```wit
interface ed25519 {
    derive-public-key: func(path: string) -> list<u8>;
    sign: func(path: string, msg: list<u8>) -> list<u8>;
}
interface secp256k1 {
    derive-public-key: func(path: string) -> list<u8>;
    sign: func(path: string, prehash: list<u8>) -> list<u8>;  // 32-byte prehash; returns r||s||v
}
```

Non-obvious:

- **Derivation is non-hierarchical.** Every key derives directly from the app root keyed by `(app_id, path)`. Children are peers, not nested.
- **Signatures are non-deterministic.** Host MAY return different signatures for the same `(path, msg)`. Don't assume equality; don't key caches on signatures.

## Constants & limits

- App identity = SHA-256 of the wasm. Same hash → same app, regardless of where it's hosted.
- Code distribution is decoupled from on-chain: bytes live anywhere the resolver can fetch (HTTPS, IPFS gateway, `data:` URI).
- Signing is host-mediated. Guests never see secret keys. `InMemorySigner` (default) holds the root key; scopes derivation by `(app_id, path)`.
- Fuel-metered. stdio limits: stdin 4 MB, stdout 4 MB, stderr 16 KB — constants in `executor/src/lib.rs`.
- Compiled components cached by code hash (moka).

## When editing

- `wit/` → regenerate `sys`, update `host`, re-check examples.
- Host import semantics (derivation, signature format) → coordinate with every deployed guest; their on-chain `code_hash` won't update for you.
- stdio/fuel constants → check callers in `service/`.
- Run: `make defuse-outlayer-app && cargo integration-tests outlayer`.

# defuse-outlayer-app

On-chain piece of Outlayer. Per-app config contract deployed as one [NEP-591](https://github.com/near/NEPs/blob/master/neps/nep-0591.md) global instance per app, at a [NEP-616](https://github.com/near/NEPs/blob/master/neps/nep-0616.md) deterministic AccountId derived from its `StateInit`.

State: `{ admin_id, code_hash, code_url }`. Same triple → same address.

For the full Outlayer stack (off-chain runtime, WIT, service/executor), see `crates/outlayer/.claude/CLAUDE.md`. This file covers only the on-chain contract.

Public API (`oa_set_code`, `oa_transfer_admin`, view methods, events) is documented in `README.md`.

## Build / test

```bash
make defuse-outlayer-app                # → res/defuse-outlayer-app.wasm
cargo integration-tests outlayer
```

Loaded as `OUTLAYER_APP_WASM` (gated on the `outlayer` feature in `crates/testing/utils/src/wasms.rs`).

## `near oa` CLI extension

`examples/near-oa.rs` is a [near-cli-rs](https://github.com/near/near-cli-rs) extension that computes `StateInit` JSON from `(admin_id, code_hash, code_url)`:

```bash
cargo install --path . --example near-oa   # then `near oa ...` works
```

## Invariants

- Contract is intentionally tiny — only stores `(wasm location, expected hash)`. All app logic is off-chain.
- AccountId is derived from `StateInit`. Fields can be mutated via `oa_set_code` / `oa_transfer_admin`, but the **address is fixed forever** by the initial triple.
- The off-chain service (`crates/outlayer/service`) reads this contract, fetches `code_url`, verifies SHA-256, runs in a sandbox.

# defuse-global-deployer

Two-step deployer for global contract code on NEP-616 deterministic accounts. Implements the upgrade mechanism for [NEP-591](https://github.com/near/NEPs/blob/master/neps/nep-0591.md).

See `README.md` for the deployment protocol; `examples/` for near-cli usage.

## Flow

1. `gd_approve(code_hash)` — **owner-only** (typically a DAO). Approves a hash.
2. `gd_deploy(wasm, storage_deposit)` — **anyone** can submit matching WASM; contract verifies the hash and deploys. Caller pays storage.

Separates governance (which hash) from operations (paying gas + storage).

## Build / test

```bash
make defuse-global-deployer             # → res/defuse-global-deployer.wasm
cargo integration-tests deployer
```

Loaded as `DEPLOYER_WASM` (`crates/testing/utils/src/wasms.rs`).

## When you change this

Many other contracts deploy *through* this one (`defuse-outlayer-app`, `defuse-escrow-swap`, …). Run the full integration-test suite, not just `deployer`.

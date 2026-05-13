# defuse-wallet

Generic minimalistic wallet for **sharded, extendable account abstraction** on NEAR. `README.md` covers promise DAGs, extensions, nonces, key rotation.

> ⚠️ **NOT AUDITED.** Do not custody significant funds.

## Build / test

```bash
make defuse-wallet                  # → res/defuse-wallet.wasm
cargo integration-tests wallet      # depends on deployer
```

Loaded as `WALLET_WASM` (`crates/testing/utils/src/wasms.rs`).

## Invariants (preserve when editing)

- **One signing scheme per deploy.** Each scheme (`p256`, `ed25519`, …) is a separate global contract — don't merge.
- **Public key is fixed at first init.** Can be *disabled* (extensions take over rotation); slot can't be repointed.
- **AccountId derives from init state** including `subwallet_id` — one key → many wallets by varying that field.
- **Optimized for ZBAs (≤770 bytes, NEP-448).** Don't break that without weighing onboarding-without-NEAR cost.
- **No self-calls inside promise DAGs.** Security invariant.
- **Lockout protection:** cannot disable signature AND have zero extensions simultaneously.
- **Nonces are non-sequential, double-timeout-window.** `wallet.timeout` is init-fixed and gates storage usage.

## Related

- `crates/wallet/sdk` — client-side SDK (request construction, signing, proof generation). Has `ed25519` feature.
- `crates/wallet/relayer` — relayer reference impl. Reads wasm via `DEFUSE_USE_OUT_DIR` (default `./res`); see `examples/relay.rs`.
- `crates/signatures/*` — per-scheme `Payload`/`SignedPayload` the wallet dispatches to.

Changing the wallet's request envelope → update SDK and relayer too.

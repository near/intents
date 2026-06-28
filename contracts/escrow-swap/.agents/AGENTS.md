# defuse-escrow-swap

Deterministic ([NEP-616](https://github.com/near/NEPs/blob/master/neps/nep-0616.md)) escrow-swap. Maker locks `src_token`; (whitelisted or permissionless) taker fills with `dst_token`. All params are immutable at init — same params → same AccountId.

See `README.md` for parameter schema and on-chain semantics.

## Build / test

```bash
make defuse-escrow-swap                 # → res/defuse-escrow-swap.wasm
cargo integration-tests escrow-swap
```

Tests load as `ESCROW_SWAP_WASM` (`crates/testing/utils/src/wasms.rs`).

## Features

- `auth_call` (default in tests) — exposes auth-call entry point so a separate authenticator (e.g. wallet) can drive the swap.

# PoA bridge contracts

Proof-of-Authority bridge for moving tokens from external chains (BTC, ETH, SOL, …) onto NEAR so they can flow through the Verifier. Predates Omni-Bridge / Chain Signatures; still in production.

## `factory/` — `defuse-poa-factory`

Spawns `poa-token` instances per bridged asset; holds bridge signer keys; per-asset tokens delegate mint checks back here.

⚠️ `factory/build.rs` **embeds the token wasm into the factory binary**. Editing anything under `token/` requires rebuilding the **factory** wasm too — otherwise the factory ships stale token bytecode.

## `token/` — `defuse-poa-token`

Bridged-asset NEP-141 FT. Mint/burn must originate from the factory. Built standalone via `cargo near build`, also baked into the factory at compile time.

## Build / test

```bash
make defuse-poa-factory       # → res/defuse-poa-factory.wasm (factory + embedded token)
cargo integration-tests poa
```

Loaded as `POA_FACTORY_WASM` (`crates/testing/utils/src/wasms.rs`).

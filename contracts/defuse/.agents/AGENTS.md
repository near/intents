# defuse — the Verifier

Core of NEAR Intents. Deployed at `intents.near`. Custodies balances, verifies/matches signed intents, executes atomically. Largest contract here.

References: `README.md` (salts, nonces), [cargo docs](https://near.github.io/intents/) (Rust API), `CHANGELOG.md` (state-migration impact).

## Layout

- `src/` — entrypoints, execution, state, token receivers
- `core/` — `defuse-core`: pure verification + matching, no runtime deps

## Variants

| Command           | Features       | Output                |
| ----------------- | -------------- | --------------------- |
| `make defuse`     | `contract`     | `res/defuse.wasm`     |
| `make defuse/far` | `contract,far` | `res/defuse.far.wasm` |
| `make defuse/all` | both           |                       |

`far` adds sandbox/test hooks. Tests load both via `DEFUSE_WASM` / `DEFUSE_FAR_WASM` (`crates/testing/utils/src/wasms.rs`).

## Signature schemes

Each at `crates/signatures/<scheme>/`, implementing `Payload` / `SignedPayload` from `defuse-crypto`:
NEP-413, NEP-461, ERC-191, BIP-322, TIP-191, WebAuthn, SEP-53, TonConnect.

New scheme: implement there, then wire into the Verifier's signature dispatch.

## Tests

Most of `tests/` exercises the Verifier — rebuild wasm before re-running, otherwise stale bytecode silently passes:

```bash
make defuse/all && cargo integration-tests defuse
DEFUSE_MIGRATE_FROM_LEGACY=1 make test   # state-migration vs releases/previous.wasm
```

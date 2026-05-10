# Treasury Logger Contract

A passive NEAR contract that receives multi-token transfers and emits
structured events for off-chain indexing. It accepts every incoming transfer
(returns a zero refund), records a monotonically increasing nonce, and never
exposes the funds back on-chain — making it a write-only audit log of
deposits into a treasury account.

## Behavior

The contract implements one standard receiver interface:

- **NEP-245** `mt_on_transfer` — invoked by multi-token contracts during
  `mt_transfer_call`. Emits `mt_deposit` and returns a vector of zeros of
  the same length as `amounts` (keeps the full transferred amounts).

The call panics with `"token_ids and amounts length mismatch"` when the
two input vectors have different lengths. The nonce is incremented after
the event is emitted; on `u128` overflow the call panics with
`"nonce overflow"`.

The original `sender_id` and `previous_owner_ids` are intentionally **not**
stored or logged — the only identity recorded is the calling token contract
(`env::predecessor_account_id()`), surfaced as the event's `token` field.
Indexers that need the original sender must parse the accompanying `msg`
payload or correlate against the token contract's own transfer event.

Note that `mt_on_transfer` is a public method and can be invoked directly
without going through `mt_transfer_call`. The contract performs no
verification that an actual transfer occurred, so emitted events should be
treated as *claims* by the predecessor account rather than proof of
deposit; correlate them against the token contract's own transfer event
when authenticity matters.

## State

```rust
pub struct Contract {
    nonce: u128,
}
```

Initialized to `0` by `new()`. There is no owner, admin, or pause switch —
the contract's only mutating entry point is the single receiver hook.

## View methods

### `get_nonce`

```rust
pub fn get_nonce(&self) -> U128
```

Returns the current value of `nonce` (the value that the *next* emitted
event will carry), serialized as a decimal string.

## Events

Events are emitted under the `logger` standard, version `1.0.0`, in the
NEP-297 `EVENT_JSON:…` log format.

### `mt_deposit`

```json
{
  "standard": "logger",
  "version": "1.0.0",
  "event": "mt_deposit",
  "data": {
    "token": "<mt-contract-account-id>",
    "token_ids": ["<token-id>", "..."],
    "amounts": ["<u128 as string>", "..."],
    "msg": "<string passed to mt_transfer_call>",
    "nonce": "<u128 as string>"
  }
}
```

`token_ids` and `amounts` are positionally aligned and have equal length.
`nonce` is serialized as a decimal string (NEAR's `U128` JSON
representation), starting at `"0"` for the first emitted event.

## Building

The crate is configured for reproducible builds via `cargo near`:

```bash
cargo near build non-reproducible-wasm --locked --no-embed-abi
```

The build image and digest are pinned in `Cargo.toml` under
`[package.metadata.near.reproducible_build]`.

## Testing

```bash
cargo test -p treasury-logger
```

Unit tests in `src/lib.rs` assert the exact event JSON produced by the
receiver path.

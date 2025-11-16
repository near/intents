# Defuse Escrow Smart Contract

Deterministic escrow contract that holds one maker order and lets takers fill it via FT (`NEP-141`) or MT (`NEP-245`) transfers. The contract keeps all settlement parameters immutable by hashing them into storage at initialization and exposes explicit methods to close and sweep the escrow. This document describes the contract behavior and provides a detailed specification of every public method and its parameters.

## Highlights
- **Deterministic deployment** – the factory derives the escrow account id from the serialized parameters (including `salt`).
- **Composable tokens** – supports both fungible (`NEP-141`) and multi-token (`NEP-245`) assets through `defuse-token-id`.
- **Single maker vs many takers** – maker locks `src_token` in the escrow until filled or closed; takers simply send `dst_token` at or above an agreed price to receive the maker's `src_token`.
- **Strict parameter verification** – every call supplies the same `Params` struct; mismatches return `Error::InvalidData`.
- **Graceful recovery** – `escrow_close` and `escrow_lost_found` distribute any remaining maker funds and optionally delete the account when balances reach zero.
- **Event rich** – emits `escrow-swap` standard events for creation, funding, fills, losses, closes, and cleanup.

## Parameter Reference

All user-facing entrypoints accept a canonical `Params` struct. The struct is serialized (Borsh) and hashed inside storage; each call re-validates the hash to prevent tampering.

### `Params` JSON Template

Use the commented JSON template below (JSONC) as a reference—each field is documented inline to keep the spec and the data definition in one place. Every call must resend the exact same serialized data so the stored hash matches.

```jsonc
{
  // owner that funds src_token and authorizes close when inventory is empty
  "maker": "maker.near",

  // primary use case: intents.near wrapping another token; canonical NEP-245 string
  "src_token": "nep245:intents.near:nep141:btc.omft.near",

  // taker provides another intents.near-wrapped asset to receive the maker's src_token
  "dst_token": "nep245:intents.near:nep141:wrap.near",

  // maker wants to receive at least 28,250.1234 dst per 1 src
  "price": "28250.1234",

  // RFC3339 timestamp string; fills after this instant fail and anyone may close
  "deadline": "2024-07-09T00:00:00Z",

  // allow takers to consume only part of maker_src_remaining
  "partial_fills_allowed": true,

  // optional overrides for returning unfunded src_token after close
  "refund_src_to": {
    // optional receiver for refunds once the escrow is closed
    "receiver_id": "maker.vault.near",

    // optional memo forwarded to the token contract
    "memo": "escrow-refund-42",

    // using mt_transfer_call to refund dst_token with a payload
    "msg": "MESSAGE",

    // optional minimum gas (in yocto-gas) reserved for the outgoing transfer
    "min_gas": "25000000000000"
  },

  // optional overrides for where maker receives dst_token during fills
  "receive_dst_to": {
    // optional override for where fills deliver dst_token
    "receiver_id": "maker.treasury.near",

    // optional memo forwarded to dst token contract
    "memo": "escrow-fill-42",

    // using mt_transfer_call to forward dst_token with a payload
    "msg": "MESSAGE",

    // optional minimum gas reserved for the outgoing transfer
    "min_gas": "40000000000000"
  },

  // optional set limiting who can fill; a single taker may close early
  "taker_whitelist": ["solver-bus-proxy.near"],

  // pct-based protocol fees
  "protocol_fees": {
    // 2.50% of taker_dst_used sent to collector
    "fee": "250",

    // 0.50% of price improvement above maker price
    "surplus": "50",

    // destination for protocol fees
    "collector": "protocol.near"
  },

  // additional fee share paid in dst_token
  "integrator_fees": {
    // 1% fee share sent to partner.near
    "partner.near": "100"
  },

  // optional contract allowed to call on_auth
  "auth_caller": "intents.near",

  // 32-byte hex used for deterministic account id derivation
  "salt": "9e3779b97f4a7c1552d27dcd1234567890abcdef1234567890abcdef1234"
}
```

> **NEP-245 serialization:** when Params are serialized (e.g., hashed by the factory), each NEP-245 token becomes the canonical string `nep245:<CONTRACT_ID>:<TOKEN_ID>` defined in `defuse-token-id`. Reuse exactly the same casing and separators or the on-chain `params_hash` equality check will fail.

### `OverrideSend`

All fields in `OverrideSend` are optional; leaving the entire object empty (or omitting it) makes the contract use its built-in defaults.

Field | Description
--- | ---
`receiver_id` | Override for the default receiver (`maker` or taker). Fallbacks: maker for dst, sender for src.
`memo` | Optional NEAR FT/MT `memo`/`msg` field.
`msg` | When set, the contract uses `transfer_call`. Failing `*_transfer_call` **does not** trigger refunds.
`min_gas` | Minimum gas to reserve for the outgoing transfer. If lower than the token's minimum, the contract automatically bumps it.

### Fees
- `ProtocolFees { fee, surplus, collector }` takes two pips amounts:
  - `fee` applies to `taker_dst_used`.
  - `surplus` applies to any price improvement (`taker_dst_used - maker_price`). Both are capped so total fees ≤ 25% (`Error::ExcessiveFees`).
- `integrator_fees` is a map of `AccountId -> Pips` capped by the same aggregate limit.
- Fees are paid in `dst_token` during fills and incur separate transfers. Ensure every collector has enough storage on the token contract.

## Transfer Message Reference

Token transfers land in `ft_on_transfer`/`mt_on_transfer` with a UTF-8 JSON message that wraps a `TransferMessage`:

```json
{
  "params": { /* full Params serialized as JSON */ },
  "action": "fund"               // or {"fill": {...}} (see below)
}
```

### Funding
Makers fund the escrow with `ft_transfer_call` or `mt_transfer_call`:

```json
{
  "params": { ... },
  "action": "fund"
}
```
- Token must equal `params.src_token`.
- `sender_id` must match `params.maker`.
- Amount contributes to `maker_src_remaining`.

### Filling
Takers send `dst_token` directly to the escrow contract via `*_transfer_call` on `params.dst_token` with:

```json
{
  "params": { ... },
  "action": {
    "fill": {
      "price": "1.045",
      "receive_src_to": {
        "receiver_id": "taker.alt.near",
        "memo": "order-42",
        "msg": null,
        "min_gas": "20000000000000"
      }
    }
  }
}
```
- `price` must be ≥ maker price.
- `receive_src_to` overrides where the swapped `src_token` goes.
- If the taker sends more `dst_token` than required, the unused portion is returned via the `return_value` helper (so the FT resolves naturally).
- Maker funds remain locked inside the escrow until a fill or close; a sole whitelisted taker may close (cancel) the escrow before the deadline to unlock the maker's tokens.

## Contract Lifecycle & Methods

### Initialization: `escrow_init(params: &Params) -> Contract`
- **Access**: called once during deployment (usually by a factory).
- **Validation**: checks `params.validate()` (distinct tokens, non-zero price, acceptable gas budget and fees).
- **State**:
  - Persists `params_hash`.
  - Initializes `maker_src_remaining = 0`, `maker_dst_lost = 0`, `deadline`, and flags.
  - Emits `Event::Create`.
- **Safety**: also ensures the newly created account id matches `Storage::derive_account_id` (factory sanity check).

### Token Receivers: `ft_on_transfer` & `mt_on_transfer`
- **Triggers**: NEP-141 `ft_transfer_call` or NEP-245 `mt_transfer_call`.
- **Arguments**: `(sender_id, amount, msg)` plus token metadata via `env::predecessor_account_id`.
- **Logic**:
  - Parses `TransferMessage`.
  - Rejects with `Error::Closed` if the escrow is closed or deadline elapsed.
  - Dispatches to `State::on_fund` or `State::on_fill`.
  - Funds and fills return `PromiseOrValue<u128>` that NEAR uses to determine refunds (unused `dst_token` is refunded to the taker).

### View: `escrow_view(&self) -> Storage`
- **Access**: view-only.
- **Returns**: full `Storage` struct:
  - `params_hash` – keccak256 hash of the canonical Params.
  - `state` – includes `maker_src_remaining`, `maker_dst_lost`, `deadline`, `closed`, `in_flight`.
- **Usage**: indexers compare `params.hash()` with `storage.params_hash` to verify they operate against the right parameters.

### Closing: `escrow_close(params: Params) -> PromiseOrValue<bool>`
- **Access**: 
  - Maker when `maker_src_remaining == 0`.
  - Any account once `deadline` expired.
  - The single taker in `taker_whitelist` (when the set has exactly one element).
- **Effects**:
  - Marks the escrow as `closed`.
  - Triggers `lost_found` internally to send leftover maker funds.
  - Returns `true` if the contract storage was cleaned up (i.e. deleted) as part of the call; otherwise returns/propagates the Promise chain that settles transfers.
- **Errors**: `Error::Unauthorized` when caller doesn't satisfy the above, `Error::InvalidData` when params mismatch.

### Sweeping: `escrow_lost_found(params: Params) -> PromiseOrValue<bool>`
- **Access**: anyone (permissionless) can retry payouts after the escrow is already closed.
- **Behavior**:
  - Attempts to send any remaining `maker_src_remaining` (only possible after close) to `refund_src_to.receiver_id` or maker.
  - Sends accumulated `maker_dst_lost` (from failed maker receipts) to `receive_dst_to.receiver_id` or maker.
  - Chains callbacks to `escrow_resolve_transfers` to account for partial refunds.
  - Returns `true` once the account met cleanup conditions (closed + zero balances + no callbacks).

### Optional Auth Call: `on_auth(signer_id: AccountId, msg: String) -> PromiseOrValue<()>`
- **Feature gate**: requires `auth_call`.
- **Purpose**: integrates with `defuse-auth-call` so a relayer can forward off-chain signed instructions.
- **Message format**:
  ```json
  {
    "params": { ... },
    "action": {
      "close": {
        "params": { ... }
      }
    }
  }
  ```
- **Checks**: `env::predecessor_account_id()` must equal `params.auth_caller`. The provided Params are validated the same way as other methods.

### Internal Callback: `escrow_resolve_transfers(maker_src, maker_dst) -> bool`
- **Trigger**: private callback after any outbound transfer.
- **Responsibility**: inspects promise results, credits unrecovered amounts to `maker_dst_lost`/`maker_src_remaining`, emits `Event::MakerLost`, and attempts cleanup.

## Events
Events follow the `escrow-swap` standard (`standard: "escrow-swap"`):
- `Event::Create` – emitted on `escrow_init`.
- `Event::Funded` – maker added liquidity.
- `Event::Fill` – taker fill succeeded, including fee breakdowns.
- `Event::MakerLost` – records assets that could not be delivered and now sit in `lost_found`.
- `Event::Closed` – indicates why the escrow shut down (`deadline_expired`, `by_maker`, `by_single_taker`).
- `Event::Cleanup` – contract deleted itself (no assets left).

## Error Codes
Error | Meaning
--- | ---
`Closed` | Escrow already closed or deadline passed when attempting to fund/fill.
`CleanupInProgress` | Cleanup guard has removed storage; no further actions allowed.
`DeadlineExpired` | Deadline reached during initialization or fill.
`ExcessiveFees` | Sum of `protocol_fees` and `integrator_fees` exceeds 25% or transfer gas budget is too high.
`ExcessiveGas` | Computed gas requirements exceed `300 Tgas - allowances`.
`IntegerOverflow` | Arithmetic overflow while computing payouts.
`InsufficientAmount` | Transfer amount too small for the requested action.
`InvalidData` | Supplied Params do not match the stored hash.
`JSON` | Malformed JSON in transfer messages or auth calls.
`PartialFillsNotAllowed` | Maker disabled partial fills but taker attempted one.
`PriceTooLow` | Maker price was zero during init or taker price < maker price.
`SameTokens` | `src_token` equals `dst_token`.
`Unauthorized` | Caller not permitted (closing, funding, etc.).
`WrongToken` | Token received on the wrong side (e.g., funding with `dst_token`).

## Cleanup Semantics
- Every outbound transfer increments `state.in_flight`. Callbacks decrement it and attempt cleanup.
- Cleanup deletes the contract (via `Promise::delete_account`) and transfers remaining NEAR balance to `env::signer_account_id()`. Ensure relayers attach enough gas to reach the cleanup stage.
- Until cleanup completes, anyone may call `escrow_lost_found` again to retry transfers that failed due to missing storage deposits on target contracts.

## Example Flow
1. **Factory deploys** `escrow-…` account and calls `escrow_init(params)`.
2. **Maker funds** the escrow by calling `ft_transfer_call` on `src_token` with `action: "fund"`. These funds stay locked until a fill succeeds or `escrow_close` executes (deadline expiry, maker action, or sole whitelisted taker cancellation).
3. **Taker fills** by sending `dst_token` with an `action.fill` message quoting a price ≥ maker price.
4. **Contract transfers**:
   - Maker receives `dst_token` (net of fees + overrides).
   - Taker receives `src_token`.
   - Integrator/protocol collectors receive their `dst_token` shares.
   - Any unused `dst_token` is refunded via the NEP-141/245 `resolve_transfer` path.
5. **Maker or anyone closes** once the deadline hits or inventory is exhausted.
6. **Lost & Found** can be retried until all balances are drained, after which the contract emits `Cleanup` and deletes itself.

By following the method specifications and message formats above, integrators can safely interact with the escrow and build higher-level order flow or RFQ systems on top of it.

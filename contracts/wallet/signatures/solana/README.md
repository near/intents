# Solana wallet contract

NEAR wallet contract controlled by a **Solana Ed25519 account**. The account
signs the [readable signing format](../readable/README.md#format-v1) with the
wallet's `signMessage(bytes, "utf8")` API, which
[shows the UTF-8 text](https://docs.phantom.com/solana/signing-a-message) in
its prompt, so the user reads the same text that the contract rebuilds and
verifies. It is a separate variant from the digest-signing Ed25519 one and
uses neither Sign-In-With-Solana nor transaction signing.

## Identity and proof

- The wallet state stores the 32-byte Ed25519 public key; `w_public_key()`
  returns `ed25519:<base58>`. Weak (small-order) keys never verify.
- The signed text has one identity field after `signer_id`:
  `solana_public_key`, the raw base58 key as the Solana wallet reports it.
- The signature covers the text bytes as they are: no prefix, prehash, BOM or
  line-ending conversion.
- The proof is `ed25519:<base58 of the 64-byte signature>`.
- Proofs of execution requests, of NEP-641 authorizations and of the
  digest-signing variant are not interchangeable.

Before showing the derived account as a deposit destination,
[verify control](../readable/README.md#verify-control-before-funding) of the
key.

## Limitations

The limits of the text are listed with the
[format](../readable/README.md#format-v1).

A wallet may sign less than the contract accepts. Phantom signed the captured
messages of up to 4057 bytes (see
[`phantom-extension.json`](tests/fixtures/phantom-extension.json)), but opened
no prompt for a message at the 16 KiB limit and answered "You cannot sign
solana transactions using sign message". Its limit is not known: try a request
of the intended size with the target wallet.

The proof must be a signature over the text bytes themselves. A wallet or
device that signs something else for `signMessage`, such as the text wrapped
in the Solana off-chain message envelope, produces a proof that never
verifies. Hardware wallets were not tested.

## Build

```sh
make defuse-wallet-solana
```

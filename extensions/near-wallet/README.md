# `near wallet`

`near-wallet` is a [near-cli-rs extension](https://github.com/near/near-cli-rs/tree/main/extensions).
It converts wallet-contract `RequestMessage` JSON to the deterministic NEP-413
payload used by the Ed25519 NEP-413 wallet variant, delegates signing to
near-cli-rs, and emits JSON arguments ready for `w_execute_signed`.

The extension deliberately does not depend on near-cli-rs internals. `near`
discovers a `near-wallet` executable on `PATH` and runs it for `near wallet`.
For signing, this extension invokes `near message sign-nep413`, preserving
near-cli's keychain, access-key file, seed phrase, private-key, and Ledger
backends.

Its command tree uses the same
[`interactive-clap`](https://docs.rs/interactive-clap/0.3.2/interactive_clap/)
flow as near-cli-rs. Run it without arguments to select an action and answer
prompts, or provide every argument for script-friendly operation:

```sh
near wallet
near wallet sign
```

In interactive mode, Escape goes back from the current item, Ctrl-C aborts the
command, and signing-backend menus support Back navigation. Fully specified
commands do not prompt; their stdout is JSON only.

## Install

Install near-cli-rs first, then install the extension from this repository:

```sh
cargo install --path ./extensions/near-wallet
near wallet --help
```

## Request message input

Both commands accept a complete wallet `RequestMessage` as inline JSON, from a
file prefixed with `@`, or from stdin as `@-`:

```json
{
  "chain_id": "mainnet",
  "signer_id": "0s0123456789abcdef0123456789abcdef01234567",
  "nonce": 42,
  "created_at": "2026-08-08T10:00:00Z",
  "timeout_secs": 3600,
  "request": {}
}
```

`created_at` should normally lag the current time slightly (the wallet SDK uses
60 seconds) while remaining inside `timeout_secs`. Nonces are replay-protected;
do not reuse a nonce until at least twice the wallet timeout has elapsed.

When the positional request argument is omitted, both `payload` and `sign`
offer a request-source menu:

1. Use existing JSON (inline, `@FILE`, or `@-`).
2. Build a new `RequestMessage` interactively.

The builder prompts for the chain ID, wallet-contract signer ID, nonce,
timeout, and creation time. Matching the wallet SDK, the creation-time default
lags the current time by the smaller of 60 seconds or one-fifth of the timeout;
the timeout default is 3600 seconds. `pay_for_gas` remains `false` because
wallet-paid gas is not currently supported by the contract.

Request contents are assembled through repeatable menus. Internal operations
cover signature-mode changes and adding/removing extensions. External promises
prompt for a receiver, optional refund account, and one or more ordered actions:

- function call, with arguments supplied as none, JSON, UTF-8, or base64;
- NEAR transfer;
- deterministic state initialization, available only as the promise's first
  action.

All internal operations execute before any external promises, and their
validity can depend on the wallet's current signature/extension state. Token
and gas prompts use the unit-aware parsers from the NEAR crates, for
example `1 NEAR`, `1 yoctoNEAR`, or `50 Tgas`; bare integers are rejected.
Separate external promises fan out independently, while actions inside each
promise preserve their entered order. Wallet self-calls are rejected.

Deterministic initialization accepts typed `StateInit` JSON inline or from an
`@FILE`. This supports both global-code identifier variants and base64 state
data without an unwieldy byte-by-byte prompt. For example:

```json
{"V1":{"code":{"account_id":"global.near"},"data":{}}}
```

The promise receiver must equal the account ID derived from that state init.
Before continuing to payload generation or signing, the builder prints the
complete `RequestMessage` to stderr and asks for confirmation. Existing fully
specified commands retain their original syntax and JSON-only stdout.

## Inspect or export the NEP-413 payload

```sh
near wallet payload @request-message.json --pretty
```

The output is the exact NEP-413 payload a compatible signer must sign. The
optional `--callback-url` is intended only for external/manual signing. The
wallet contract reconstructs a callback-less payload, so a callback-bearing
signature cannot be submitted to `w_execute_signed`.

## Sign through near-cli-rs

`sign` first gathers the request, `--sign-as` account, and output options, then
opens an interactive signing-backend menu. `--sign-as` is the ordinary NEAR
account whose access key near-cli should use; it is distinct from the wallet
contract account in `msg.signer_id`.

For example, using a key stored in near-cli's secure keychain:

```sh
near wallet sign @request-message.json \
  --sign-as alice.near \
  --expected-public-key ed25519:3KyWn9... \
  sign-with-keychain mainnet
```

Or use an access-key file:

```sh
near wallet sign @request-message.json \
  --sign-as alice.near \
  --expected-public-key ed25519:3KyWn9... \
  sign-with-access-key-file ~/.near-credentials/mainnet/alice.near.json
```

The interactive menu mirrors near-cli's current signing modes:

- `sign-with-keychain NETWORK`
- `sign-with-legacy-keychain NETWORK`
- `sign-with-ledger [--seed-phrase-hd-path HD_PATH] usb`
- `sign-with-plaintext-private-key PRIVATE_KEY`
- `sign-with-access-key-file FILE`
- `sign-with-seed-phrase SEED_PHRASE [--seed-phrase-hd-path HD_PATH]`

Private keys and seed phrases use hidden terminal prompts when omitted. Passing
them on the command line may expose them through shell history or process
inspection, so the interactive prompt, keychain, or access-key file is safer.

`custom` preserves compatibility with newer or unusual near-cli backends. Its
single value is shell-parsed into an argument vector but is still executed
directly, never through a shell:

```sh
near wallet sign @request-message.json --sign-as alice.near \
  custom 'sign-with-keychain network-config mainnet'
```

The signing subprocess's stderr and stdin stay attached to the terminal for
prompts. Its stdout is captured and verified locally. On success, this
extension's stdout contains only the contract arguments:

```json
{"msg":{"chain_id":"mainnet","signer_id":"0s...","nonce":42,"created_at":"2026-08-08T10:00:00Z","timeout_secs":3600,"request":{}},"proof":"ed25519:..."}
```

This can be piped into a relayed contract call:

```sh
SIGNED_ARGS="$(near wallet sign @request-message.json \
  --sign-as alice.near \
  --expected-public-key ed25519:3KyWn9... \
  sign-with-keychain mainnet)"

near contract call-function as-transaction \
  0s0123456789abcdef0123456789abcdef01234567 \
  w_execute_signed json-args "$SIGNED_ARGS"
```

Continue through near-cli's prompts to choose gas, the relayer account, network,
and transaction-signing method. The relayer does not need to be the NEP-413
signer.

### Signing safety

- `sign` always uses `callback_url: null`, matching contract verification.
- Arguments are passed directly to the child process without a shell.
- The returned account ID, Ed25519 encodings, and NEP-413 signature are checked
  locally before `w_execute_signed` JSON is emitted.
- `--expected-public-key` additionally protects accounts with multiple
  full-access keys from accidentally selecting a key different from the one
  fixed in wallet state. Supplying it is strongly recommended.
- If near-cli exits unsuccessfully, the extension exits without emitting
  partial contract arguments.

The legacy spelling `--signer-account-id` remains an alias for `--sign-as`.

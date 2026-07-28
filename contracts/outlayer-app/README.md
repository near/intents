# Outlayer App

A per-app code configuration contract, deployed as one [NEP-591 global contract](https://github.com/near/NEPs/blob/master/neps/nep-0591.md) instance per app. Each instance tracks a code URL and approved hash — serving as a canonical on-chain reference for where an app's contract code lives and how to fetch it.

## Overview

An Outlayer App instance is a [NEP-616 deterministic account](https://github.com/near/NEPs/blob/master/neps/nep-0616.md) derived from its `StateInit`. The `StateInit` encodes the complete initial contract state, uniquely identifying the instance. Differences in `admin_public_key`, `state`, or `config` produce different addresses even when `admin_id`, `code_hash`, and `code_url` match.

One admin account manages the instance: it approves the expected code hash and sets the URL where the binary can be fetched. The URL can be an HTTPS link or a `data:` URI embedding the binary inline.

## Contract State

| Field              | Type                          | Default       | Description                                          |
|--------------------|-------------------------------|---------------|------------------------------------------------------|
| `admin_id`         | `AccountId`                   | set at init   | Account authorized to approve and configure          |
| `code_hash`        | `[u8; 32]`                    | set at init   | SHA-256 of the approved code binary                  |
| `code_url`         | `Url`                         | set at init   | URL where the code binary can be fetched             |
| `admin_public_key` | `AdminPublicKey`              | set at init   | Admin's public key (ed25519)                         |
| `state`            | `BTreeMap<Vec<u8>, Vec<u8>>`  | empty         | Arbitrary encrypted key-value blobs                  |
| `config`           | `BTreeMap<Vec<u8>, Vec<u8>>`  | empty         | Arbitrary config key-value pairs                     |

## StateInit Parameters

The deterministic address is derived from the Borsh-serialized `State`. All parameters must be provided at init time:

- **`admin_id`** *(required)* — account that controls code approval and configuration
- **`code_hash`** *(required)* — SHA-256 hash of the approved code binary (`[0u8; 32]` if no code is pre-approved)
- **`code_url`** *(required)* — URL pointing to the code binary (`https://...` or `data:application/wasm;base64,...`)
- **`admin_public_key`** *(required)* — admin's ed25519 public key (`ed25519:...`)

Use the [`near-oa`](#near-oa-cli-tool) tool to compute the `StateInit` JSON for a given set of parameters.

> [!IMPORTANT]
> `State::state_init()` serializes the complete `State`, so initial `state` and
> `config` entries become part of the deterministic account address. In most
> deployments, leave both maps empty so application data and configuration do
> not become additional inputs to the instance's identity. Set them during
> deployment only when you intentionally want their contents to affect the
> resulting address.
>
> Post-deployment `set_state` and `set_config` methods are planned but not yet
> available. Once implemented, they can be chained as promises to the
> `StateInit` deployment transaction, keeping runtime state and configuration
> out of the address derivation.

## Public API

### `oa_set_code(code_hash, code_url)`
Atomically sets the approved SHA-256 hash and the code URL. Admin-only, requires at least 1 yoctoNEAR. Emits `SetCode`.

### `oa_transfer_admin(new_admin_id)`
Transfers control to a new admin. Admin-only, requires 1 yoctoNEAR. Emits `TransferAdmin`.

### `oa_set_admin_public_key(new_admin_public_key)`
Sets the admin public key. Admin-only, requires exactly 1 yoctoNEAR. Emits `SetAdminPublicKey`.

### View methods
- `oa_admin_id()` — current admin
- `oa_admin_public_key()` — current admin public key
- `oa_code_hash()` — approved hash (hex)
- `oa_code_url()` — current code URL

## Events

All events follow [NEP-297](https://github.com/near/NEPs/blob/master/neps/nep-0297.md) with standard `"near-outlayer-app"` version `"1.0.0"`.

| Event               | Fields                         | Description                        |
|---------------------|--------------------------------|------------------------------------|
| `SetCode`           | `hash`, `url`                  | Code URL and approved hash updated |
| `TransferAdmin`     | `old_admin_id`, `new_admin_id` | Admin transferred                  |
| `SetAdminPublicKey` | `new_admin_public_key`         | Admin public key updated           |

## `near oa` extension

The `near-oa` command is an extension for [near-cli-rs](https://github.com/near/near-cli-rs) that computes the `StateInit` for a outlayer-app contract, outputting a JSON map of base64-encoded key-value pairs.

### Install

```sh
cargo install --path ./crates/outlayer-app/near-oa
```

### Running

```sh
near oa [OPTIONS] --admin-id <AccountId> --code-url <URL> --code-hash <HASH> --admin-public-key <PublicKey>
```

### Usage

```sh
$ near oa --help
Print JSON storage key-value pairs (as base64) for `StateInit` of a Outlayer App contract

Usage: near-oa [OPTIONS] --admin-id <AccountId> --code-url <URL> --code-hash <HASH | @FILE | @-> --admin-public-key <PublicKey>

Options:
      --admin-id <AccountId>
          Admin account ID (controls code approval and configuration)

      --code-url <URL>
          URL where the code binary can be fetched from (e.g. `https://...` or
          `data:application/wasm;base64,...`)

      --code-hash <HASH | @FILE | @->
          SHA-256 hash of the approved code.

          `HASH` can be encoded as base58 or hex with `0x` prefix. `@FILE` will calculate SHA-256
          hash of the `FILE` contents. `@-` will calculate SHA-256 hash of the stdin contents.

      --admin-public-key <PublicKey>
          Admin's public key (e.g. `ed25519:...`)

  -q, --quiet
          Output single-line JSON only (no human-readable annotations)

  -h, --help
          Print help (see a summary with '-h')
```

### Example

```bash
near oa \
  --admin-id alice.near \
  --code-hash 0xfaf9e8500fdf8021ed8b3390580bbc86faf9e8500fdf8021ed8b3390580bbc80 \
  --code-url https://example.com/contract.wasm \
  --admin-public-key ed25519:5TagutioHgKLh7KZ1VEFBYfgRkPtqnKm9LoMnJMJugxm
```
```text
// State:
{
  "admin_id": "alice.near",
  "code_hash": "faf9e8500fdf8021ed8b3390580bbc86faf9e8500fdf8021ed8b3390580bbc80",
  "code_url": "https://example.com/contract.wasm",
  "admin_public_key": "ed25519:5TagutioHgKLh7KZ1VEFBYfgRkPtqnKm9LoMnJMJugxm",
  "state": {},
  "config": {}
}

// Storage key-value pairs (as base64):
{"":"..."}
```

Pre-approving a hash at deployment time:

```bash
near transaction construct-transaction <admin-id> \
  state-init use-global-hash <global-contract-code-hash> \
  data-from-json "$(near-oa \
    --admin-id <admin-id> \
    --code-url <url> \
    --code-hash <code-sha256-hex> \
    --admin-public-key <admin-public-key> \
    --quiet)" \
  deposit 0NEAR \
  skip \
  network-config testnet \
  sign-with-keychain
```

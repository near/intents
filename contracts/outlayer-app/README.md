# Outlayer App

A per-app code configuration contract, deployed as one [NEP-591 global contract](https://github.com/near/NEPs/blob/master/neps/nep-0591.md) instance per app. Each instance tracks a code URL and approved hash — serving as a canonical on-chain reference for where an app's contract code lives and how to fetch it.

## Overview

An Outlayer App instance is a [NEP-616 deterministic account](https://github.com/near/NEPs/blob/master/neps/nep-0616.md) derived from its `StateInit`. The `StateInit` encodes the initial contract state, uniquely identifying the instance — two instances with the same `admin_id`, `code_hash`, and `code_url` will share the same address.

One admin account manages the instance: it approves the expected code hash and sets the URL where the binary can be fetched. The URL can be an HTTPS link or a `data:` URI embedding the binary inline.

## Contract State

| Field       | Type        | Default       | Description                                          |
|-------------|-------------|---------------|------------------------------------------------------|
| `admin_id`  | `AccountId` | set at init   | Account authorized to approve and configure          |
| `code_hash` | `[u8; 32]`  | set at init   | SHA-256 of the approved code binary                  |
| `code_url`  | `Url`       | set at init   | URL where the code binary can be fetched             |

## StateInit Parameters

The deterministic address is derived from the Borsh-serialized `State`. All parameters must be provided at init time:

- **`admin_id`** *(required)* — account that controls code approval and configuration
- **`code_hash`** *(required)* — SHA-256 hash of the approved code binary (`[0u8; 32]` if no code is pre-approved)
- **`code_url`** *(required)* — URL pointing to the code binary (`https://...` or `data:application/wasm;base64,...`)

Use the [`near-oa`](#near-oa-cli-tool) tool to compute the `StateInit` JSON for a given set of parameters.

## Public API

### `oa_set_code(code_hash, code_url)`
Atomically sets the approved SHA-256 hash and the code URL. Admin-only, requires at least 1 yoctoNEAR. Emits `SetCode`.

### `oa_transfer_admin(new_admin_id)`
Transfers control to a new admin. Admin-only, requires 1 yoctoNEAR. Emits `TransferAdmin`.

### View methods
- `oa_admin_id()` — current admin
- `oa_code_hash()` — approved hash (hex)
- `oa_code_url()` — current code URL

## Events

All events follow [NEP-297](https://github.com/near/NEPs/blob/master/neps/nep-0297.md) with standard `"near-outlayer-app"` version `"1.0.0"`.

| Event           | Fields                         | Description                       |
|-----------------|--------------------------------|-----------------------------------|
| `SetCode`       | `hash`, `url`                  | Code URL and approved hash updated |
| `TransferAdmin` | `old_admin_id`, `new_admin_id` | Admin transferred                 |


## `near-oa` CLI tool

`near-oa` computes the `StateInit` JSON for a new Outlayer App instance.

### Install

```sh
cargo install --path contracts/outlayer-app --example near-oa
```

### Running

```sh
near-oa [OPTIONS] --admin-id <AccountId> --code-url <URL>
```

### Usage

```
Compute StateInit for a near-oa contract instance

Usage: near-oa [OPTIONS] --admin-id <AccountId> --code-url <URL>

Options:
      --admin-id <AccountId>  Admin account ID (controls code approval)
      --code-url <URL>        URL where the code binary can be fetched
      --code-hash <HASH>      SHA-256 hash of the approved code (hex, with or without 0x prefix)
  -q, --quiet                 Output single-line JSON only (no human-readable annotations)
  -h, --help                  Print help
```

### Example

```bash
near-oa \
  --admin-id alice.near \
  --code-url https://example.com/contract.wasm
```
```
admin_id:            alice.near
code_hash:           0000000000000000000000000000000000000000000000000000000000000000
code_url:            https://example.com/contract.wasm
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
    --quiet)" \
  deposit 0NEAR \
  skip \
  network-config testnet \
  sign-with-keychain
```

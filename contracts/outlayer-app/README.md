# Outlayer App

A per-app code configuration contract, deployed as one [NEP-591 global contract](https://github.com/near/NEPs/blob/master/neps/nep-0591.md) instance per app. Each instance tracks a code URL and approved hash — serving as a canonical on-chain reference for where an app's contract code lives and how to fetch it.

## Overview

An Outlayer App instance is a [NEP-616 deterministic account](https://github.com/near/NEPs/blob/master/neps/nep-0616.md) derived from its `StateInit`. The `StateInit` encodes the initial contract state, uniquely identifying the instance — two instances with the same `admin_id`, `code_hash`, and `code_url` will share the same address.

One admin account manages the instance: it approves the expected code hash and sets the URL where the binary can be fetched. The URL can be an HTTPS link or a `data:` URI embedding the binary inline.

## Contract State

| Field       | Type        | Default       | Description                                          |
|-------------|-------------|---------------|------------------------------------------------------|
| `admin_id`  | `AccountId` | set at init   | Account authorized to approve and configure          |
| `code_hash` | `[u8; 32]`  | `0x000...000` | SHA-256 of the approved code binary                  |
| `code_url`  | `Url`       | set at init   | URL where the code binary can be fetched             |

## StateInit Parameters

The deterministic address is derived from the Borsh-serialized `State`. The parameters at init time are:

- **`admin_id`** *(required)* — account that controls code approval and configuration
- **`code_url`** *(required)* — URL pointing to the code binary (`https://...` or `data:application/wasm;base64,...`)
- **`code_hash`** *(optional, via `pre_approve`)* — pre-approves a SHA-256 hash so the deployment can be verified immediately

Use the [`near-op`](#near-op-cli-tool) tool to compute the `StateInit` JSON for a given set of parameters.

## Public API

### `op_approve(new_hash)`
Approves a SHA-256 hash for the code binary. Admin-only, requires 1 yoctoNEAR. Emits `Approve`.

### `op_set_code_uri(url)`
Sets the URL where the code binary can be fetched. Accepts any valid URL, including `data:` URIs for inline content. Admin-only, requires 1 yoctoNEAR. Emits `SetCodeUri`.

### `op_set_admin_id(new_admin_id)`
Transfers control to a new admin. Admin-only, requires 1 yoctoNEAR. Emits `Transfer`.

### View methods
- `op_admin_id()` — current admin
- `op_code_hash()` — approved hash (hex)
- `op_code_uri()` — current code URL

## Events

All events follow [NEP-297](https://github.com/near/NEPs/blob/master/neps/nep-0297.md) with standard `"near-outlayer-app"` version `"1.0.0"`.

| Event        | Fields                         | Description           |
|--------------|--------------------------------|-----------------------|
| `Approve`    | `code_hash`                    | Approved hash changed |
| `SetCodeUri` | `url`                          | Code URL updated      |
| `Transfer`   | `old_admin_id`, `new_admin_id` | Admin transferred     |


## `near-op` CLI tool

`near-op` computes the `StateInit` JSON for a new Outlayer App instance.

### Running

```sh
cargo run -p defuse-outlayer-app --example near-op
```

### Usage

```
Compute StateInit for an outlayer-app contract instance

Usage: near-op [OPTIONS] --admin-id <AccountId> --code-url <URL>

Options:
      --admin-id <AccountId>  Admin account ID (controls code approval)
      --code-url <URL>        URL where the code binary can be fetched
      --approve <HASH>        Pre-approve a SHA-256 code hash (hex, with or without 0x prefix)
  -q, --quiet                 Output single-line JSON only (no human-readable annotations)
  -h, --help                  Print help
```

### Example

```bash
cargo run -p defuse-outlayer-app --example near-op -- \
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
  data-from-json "$(near-op \
    --admin-id <admin-id> \
    --code-url <url> \
    --approve <code-sha256-hex> \
    --quiet)" \
  deposit 0NEAR \
  skip \
  network-config testnet \
  sign-with-keychain
```

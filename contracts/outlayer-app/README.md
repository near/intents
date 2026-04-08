# Outlayer App

A per-app code configuration contract, deployed as one [NEP-591 global contract](https://github.com/near/NEPs/blob/master/neps/nep-0591.md) instance per app. Each instance is tied to a single code binary that it stores, tracks, and makes discoverable — serving as a canonical on-chain reference for where an app's contract code lives and how to fetch it.

## Overview

An Outlayer App instance is a [NEP-616 deterministic account](https://github.com/near/NEPs/blob/master/neps/nep-0616.md) derived from its `StateInit`. The `StateInit` encodes the initial contract state, uniquely identifying the instance — two instances with the same `admin_id` and `code_hash` will share the same address.

One admin account manages the instance: it approves the expected code hash, uploads the binary, and optionally points the instance to an external location if the code is hosted elsewhere.

## Contract State

| Field       | Type                    | Default       | Description                                          |
|-------------|-------------------------|---------------|------------------------------------------------------|
| `admin_id`  | `AccountId`             | set at init   | Account authorized to approve, upload, and configure |
| `code_hash` | `[u8; 32]`              | `0x000...000` | SHA-256 of the approved code; upload must match this |
| `code`      | `LazyOption<Vec<u8>>`   | empty         | The uploaded code binary (stored on-chain)           |
| `location`  | `Option<CodeLocation>`  | `None`        | Where the code can be fetched from                   |

## StateInit Parameters

The deterministic address is derived from the Borsh-serialized `State`. The two meaningful parameters at init time are:

- **`admin_id`** *(required)* — account that controls code approval and upload
- **`code_hash`** *(optional, via `pre_approve`)* — pre-approves a SHA-256 hash so `op_upload_code` can be called immediately in the same transaction as `StateInit`, without a separate `op_approve` call

Use the [`near-op`](#near-op-cli-tool) tool to compute the `StateInit` JSON for a given set of parameters.

## Code Location

Once code is uploaded or pointed to, the instance exposes a `CodeLocation` describing where to find it:

```
CodeLocation::OnChain  { account, storage_prefix }
CodeLocation::HttpUrl  { url }
... to be extended
```

**`OnChain`** — Code is stored in the account's contract storage at the given `storage_prefix` key. This is set automatically after `op_upload_code`. The `account` field identifies which contract holds the data, and `storage_prefix` is the storage key (Borsh `Vec<u8>`, base64-encoded in JSON).

**`HttpUrl`** — Code is available at an HTTPS URL, set manually via `op_set_location`.

## Public API

### `op_approve(new_hash)`
Approves a SHA-256 hash for the next upload. Admin-only, requires 1 yoctoNEAR. Emits `Approve`.

### `op_upload_code(code)`
Uploads the raw code binary. `sha256(code)` must equal the approved `code_hash`. Sets `location` to `OnChain` pointing at this contract. Admin-only. Emits `Upload` and `SetLocation`.

### `op_set_location(location)`
Manually overrides the code location (e.g. to point at an HTTP mirror). Admin-only, requires 1 yoctoNEAR. Emits `SetLocation`.

### `op_set_admin_id(new_admin_id)`
Transfers control to a new admin. Admin-only, requires 1 yoctoNEAR. Emits `Transfer`.

### View methods
- `op_admin_id()` — current admin
- `op_code_hash()` — approved hash (hex)
- `op_code()` — uploaded code bytes as base64, or `null`
- `op_location()` — current `CodeLocation`, or `null`

## Events

All events follow [NEP-297](https://github.com/near/NEPs/blob/master/neps/nep-0297.md) with standard `"near-outlayer-app"` version `"1.0.0"`.

| Event         | Fields                          | Description              |
|---------------|---------------------------------|--------------------------|
| `Approve`     | `code_hash`                     | Approved hash changed    |
| `Upload`      | `code_hash`                     | Code binary uploaded     |
| `SetLocation` | `location`                      | Code location updated    |
| `Transfer`    | `old_admin_id`, `new_admin_id`  | Admin transferred        |

## Fetching the Code

### 1. Read the location

```bash
near view <instance-id> op_location --networkId testnet
```

Example output:
```json
{
  "OnChain": {
    "account": "0s3a8e5b3b6e5c4509d21973122263b9c354e46d9f",
    "storage_prefix": "Y29kZQ=="
  }
}
```

`storage_prefix` is base64 — `Y29kZQ==` decodes to `code`, the fixed key where the binary is stored.

### 2. Fetch via view function (recommended)

`op_code()` returns the binary as a base64 string:

```bash
near view <instance-id> op_code --networkId testnet \
  | jq -r '.' \
  | base64 -d > contract.wasm
```

### 3. Fetch from raw storage

For `OnChain` locations, the code is stored at the `storage_prefix` key under the `account` contract. The value is Borsh-encoded `Vec<u8>`: a 4-byte little-endian length prefix followed by the raw bytes. Skip those 4 bytes to get the binary:

```bash
near contract view-storage <account> \
  keys-start-with-bytes-as-base64 <storage_prefix> \
  as-json network-config testnet now
```


## `near-op` CLI tool

`near-op` computes the `StateInit` JSON for a new Outlayer App instance.

### Running

```sh
cargo run -p defuse-outlayer-app --example near-op
```

### Usage

```
Compute StateInit for an outlayer-app contract instance

Usage: near-op [OPTIONS] --admin-id <AccountId>

Options:
      --admin-id <AccountId>  Admin account ID (controls code approval)
      --approve <HASH>        Pre-approve a SHA-256 code hash (hex, with or without 0x prefix)
  -q, --quiet                 Output single-line JSON only (no human-readable annotations)
  -h, --help                  Print help
```

### Example

```bash
cargo run -p defuse-outlayer-app --example near-op -- --admin-id alice.near
```
```
admin_id:            alice.near
code_hash:           0000000000000000000000000000000000000000000000000000000000000000
{"":"CgAAAGFsaWNlLm5lYXIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAEAAAAY29kZQA="}
```

Pre-approving a hash allows uploading code in the same transaction as deployment:

```bash
near transaction construct-transaction <admin-id> \
  state-init use-global-hash <global-contract-code-hash> \
  data-from-json "$(near-op \
    --admin-id <admin-id> \
    --approve <code-sha256-hex> \
    --quiet)" \
  deposit 0NEAR \
  add-action function-call "op_upload_code" \
    file-args <contract.wasm> \
    prepaid-gas "100 Tgas" \
    attached-deposit 0NEAR \
  skip \
  network-config testnet \
  sign-with-keychain
```

The `state-init` action deploys the instance (with the hash pre-approved), and the `op_upload_code` function-call action uploads the binary — both atomically in a single transaction.

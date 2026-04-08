# Outlayer Project

A per-project WASM configuration contract, deployed as one [NEP-591 global contract](https://github.com/near/NEPs/blob/master/neps/nep-0591.md) instance per project. Each instance is tied to a single WASM binary that it stores, tracks, and makes discoverable — serving as a canonical on-chain reference for where a project's contract code lives and how to fetch it.

## Overview

An Outlayer Project instance is a [NEP-616 deterministic account](https://github.com/near/NEPs/blob/master/neps/nep-0616.md) derived from its `StateInit`. The `StateInit` encodes the initial contract state, uniquely identifying the instance — two instances with the same `updater_id` and `wasm_hash` will share the same address.

One updater account manages the instance: it approves the expected WASM hash, uploads the binary, and optionally points the instance to an external location if the WASM is hosted elsewhere.

## Contract State

| Field        | Type                    | Default       | Description                                           |
|--------------|-------------------------|---------------|-------------------------------------------------------|
| `updater_id` | `AccountId`             | set at init   | Account authorized to approve, upload, and configure  |
| `wasm_hash`  | `[u8; 32]`              | `0x000...000` | SHA-256 of the approved WASM; upload must match this  |
| `wasm`       | `LazyOption<Vec<u8>>`   | empty         | The uploaded WASM binary (stored on-chain)            |
| `location`   | `Option<WasmLocation>`  | `None`        | Where the WASM can be fetched from                    |

## StateInit Parameters

The deterministic address is derived from the Borsh-serialized `State`. The two meaningful parameters at init time are:

- **`updater_id`** *(required)* — account that controls WASM approval and upload
- **`wasm_hash`** *(optional, via `pre_approve`)* — pre-approves a SHA-256 hash so `op_upload_wasm` can be called immediately in the same transaction as `StateInit`, without a separate `op_approve` call

Use the [`near-op`](#near-op-cli-tool) tool to compute the `StateInit` JSON for a given set of parameters.

## WASM Location

Once WASM is uploaded or pointed to, the instance exposes a `WasmLocation` describing where to find it:

```
WasmLocation::OnChain  { account, storage_prefix }
WasmLocation::HttpUrl  { url }
... to be extended
```

**`OnChain`** — WASM is stored in the account's contract storage at the given `storage_prefix` key. This is set automatically after `op_upload_wasm`. The `account` field identifies which contract holds the data, and `storage_prefix` is the storage key (Borsh `Vec<u8>`, base64-encoded in JSON).

**`HttpUrl`** — WASM is available at an HTTPS URL, set manually via `op_set_location`.

## Public API

### `op_approve(new_hash)`
Approves a SHA-256 hash for the next upload. Updater-only, requires 1 yoctoNEAR. Emits `Approve`.

### `op_upload_wasm(code)`
Uploads the raw WASM binary. `sha256(code)` must equal the approved `wasm_hash`. Sets `location` to `OnChain` pointing at this contract. Updater-only. Emits `Upload` and `SetLocation`.

### `op_set_location(location)`
Manually overrides the WASM location (e.g. to point at an HTTP mirror). Updater-only, requires 1 yoctoNEAR. Emits `SetLocation`.

### `op_set_updater_id(new_updater_id)`
Transfers control to a new updater. Resets `wasm_hash` to zeros. Updater-only, requires 1 yoctoNEAR. Emits `Transfer` and `Approve`.

### View methods
- `op_updater_id()` — current updater
- `op_wasm_hash()` — approved hash (hex)
- `op_wasm()` — uploaded WASM bytes as base64, or `null`
- `op_location()` — current `WasmLocation`, or `null`

## Events

All events follow [NEP-297](https://github.com/near/NEPs/blob/master/neps/nep-0297.md) with standard `"near-outlayer-project"` version `"1.0.0"`.

| Event         | Fields                              | Description                          |
|---------------|-------------------------------------|--------------------------------------|
| `Approve`     | `code_hash`                         | Approved hash changed                |
| `Upload`      | `code_hash`                         | WASM binary uploaded                 |
| `SetLocation` | `location`                          | WASM location updated                |
| `Transfer`    | `old_updater_id`, `new_updater_id`  | Updater transferred                  |

## Fetching the WASM

### 1. Read the location

```bash
near view <instance-id> op_location --networkId testnet
```

Example output:
```json
{
  "OnChain": {
    "account": "0s3a8e5b3b6e5c4509d21973122263b9c354e46d9f",
    "storage_prefix": "d2FzbQ=="
  }
}
```

`storage_prefix` is base64 — `d2FzbQ==` decodes to `wasm`, the fixed key where the binary is stored.

### 2. Fetch via view function (recommended)

`op_wasm()` returns the binary as a base64 string:

```bash
near view <instance-id> op_wasm --networkId testnet \
  | jq -r '.' \
  | base64 -d > contract.wasm
```

### 3. Fetch from raw storage

For `OnChain` locations, the WASM is stored at the `storage_prefix` key under the `account` contract. The value is Borsh-encoded `Vec<u8>`: a 4-byte little-endian length prefix followed by the raw bytes. Skip those 4 bytes to get the binary:

```bash
near contract view-storage <account> \
  keys-start-with-bytes-as-base64 <storage_prefix> \
  as-json network-config testnet now
```


## `near-op` CLI tool

`near-op` computes the `StateInit` JSON for a new Outlayer Project instance.

### Running

```sh
cargo run -p defuse-outlayer-project --example near-op
```

### Usage

```
Compute StateInit for an outlayer-project contract instance

Usage: near-op [OPTIONS] --updater-id <AccountId>

Options:
      --updater-id <AccountId>  Updater account ID (controls WASM approval)
      --approve <HASH>          Pre-approve a SHA-256 WASM hash (hex, with or without 0x prefix)
  -q, --quiet                   Output single-line JSON only (no human-readable annotations)
  -h, --help                    Print help
```

### Example

```bash
cargo run -p defuse-outlayer-project --example near-op -- --updater-id alice.near
```
```
updater_id:          alice.near
wasm_hash:           0000000000000000000000000000000000000000000000000000000000000000
{"":"CgAAAGFsaWNlLm5lYXIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAEAAAAd2FzbQA="}
```

Pre-approving a hash allows uploading WASM in the same transaction as deployment:

```bash
near transaction construct-transaction <updater-id> \
  state-init use-global-hash <global-contract-code-hash> \
  data-from-json "$(near-op \
    --updater-id <updater-id> \
    --approve <wasm-sha256-hex> \
    --quiet)" \
  deposit 0NEAR \
  add-action function-call "op_upload_wasm" \
    file-args <contract.wasm> \
    prepaid-gas "100 Tgas" \
    attached-deposit 0NEAR \
  skip \
  network-config testnet \
  sign-with-keychain
```

The `state-init` action deploys the instance (with the hash pre-approved), and the `op_upload_wasm` function-call action uploads the binary — both atomically in a single transaction.

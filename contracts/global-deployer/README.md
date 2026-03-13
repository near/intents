# Global Deployer

A minimal contract for managing global contract code on deterministic ([NEP-616](https://github.com/near/NEPs/blob/master/neps/nep-0616.md)) accounts. It implements the upgrade mechanism for [NEP-591 Global Contracts](https://github.com/near/NEPs/blob/master/neps/nep-0591.md).

## Two-Step Deployment

Deployments are split into two steps:

1. **Approve** (`gd_approve`) — the owner (typically a DAO) votes for a specific code hash
2. **Deploy** (`gd_deploy`) — anyone can execute the deployment by submitting the matching WASM binary + storage deposit

### Why permissionless deploy?

The deploy step requires attaching the full WASM binary and a storage deposit. This is error-prone (misconfigured deposit, large transaction). By separating approval from execution, the DAO only votes for a well-known code hash (e.g. from GitHub releases), and a dedicated operator or bot handles the actual deployment mechanics.

### Design philosophy

The contract is intentionally slim and low-level. More sophisticated workflows can be built on top by composing utility contracts as owners. For example, only one hash can be approved at a time — each new approval erases the previous one. If consecutive multi-stage upgrades are needed, a utility contract can queue approvals and forward them one-by-one after each previous deployment completes:

```mermaid
flowchart LR
    DAO -- "votes for H1" --> UC[Utility Contract]
    DAO -- "votes for H2" --> UC[Utility Contract]
    DAO -- ".." --> UC[Utility Contract]
    DAO -- "votes for HN" --> UC[Utility Contract]
    UC -- "queues deployments [H1, H2, .. HN]" --> GD[Global Deployer]
```

## Contract State

| Field           | Type        | Default      | Description                                       |
|-----------------|-------------|--------------|---------------------------------------------------|
| `owner_id`      | `AccountId` | set at init  | Account authorized to approve deployments and transfer ownership |
| `code_hash`     | `[u8; 32]`  | `0x000...000` | SHA-256 hash of the currently deployed code       |
| `approved_hash` | `[u8; 32]`  | `0x000...000` | SHA-256 hash of the next approved deployment      |

## Public API

### `gd_approve(old_hash, new_hash)`

Sets the approved hash for the next deployment.

- **Access**: owner only
- **Deposit**: 1 yoctoNEAR
- **Params**: `old_hash` must match current `code_hash` (prevents stale approvals), `new_hash` is the SHA-256 of the WASM to deploy next
- **State change**: sets `approved_hash` to `new_hash`
- **Events**: `Approve { code_hash: new_hash, reason: By(caller) }`

### `gd_deploy(code)`

Deploys WASM code as a global contract on this account.

- **Access**: permissionless
- **Deposit**: enough to cover storage delta
- **Params**: `code` (raw WASM binary, passed directly without borsh length prefix) — `sha256(code)` must equal `approved_hash`
- **State change**: `code_hash = sha256(code)`, `approved_hash = 0x000...000`
- **Events**: [`Deploy { code_hash }`, `Approve { code_hash: 0x000...000, reason: Deploy(code_hash) }` ]
- **Refund**: unused deposit is returned to the caller

### `gd_transfer_ownership(receiver_id)`

Transfers contract ownership to a new account.

- **Access**: owner only
- **Deposit**: 1 yoctoNEAR
- **Params**: `receiver_id` — must differ from current owner
- **State change**: `owner_id = receiver_id`, `approved_hash = 0x000...000`
- **Events**: `Transfer { old_owner_id, new_owner_id }`, then `Approve { code_hash: 0x000...000, reason: By(new_owner_id) }`

### `gd_owner_id() → AccountId`

Returns the current owner's account ID. View method.

### `gd_code_hash() → hex string`

Returns the SHA-256 hash of the currently deployed code (or `0x000...000` if none). View method.

### `gd_approved_hash() → hex string`

Returns the currently approved hash (or `0x000...000` if none). View method.

## Events

All events follow [NEP-297](https://github.com/near/NEPs/blob/master/neps/nep-0297.md) with standard `"global-deployer"` version `"1.0.0"`.

| Event      | Fields                         | Description                                                                                                  |
|------------|---------------------------------|--------------------------------------------------------------------------------------------------------------|
| `Approve`  | `code_hash`, `reason`          | Approved hash changed                                                                                         |
| `Deploy`   | `code_hash`                    | Code was deployed                                                                                            |
| `Transfer` | `old_owner_id`, `new_owner_id` | Ownership was transferred                                                                                    |

## Deployment Flow

```mermaid
sequenceDiagram
    box Contracts
        participant Owner
        participant Caller
        participant GD as 0s1234..1234<br/>(Global Deployer)
    end
    Note over NS: 0s1234..1234 => None
    box rgb(80, 120, 180) NEAR Protocol
        participant NS as Global Contracts<br/>Namespace
    end

    Owner->>GD: gd_approve(old_hash, new_hash)
    Caller->>GD: gd_deploy(code) + deposit
    GD->>NS: deploy_global_contract_by_account_id
    Note over NS: 0s1234..1234 => code
    GD->>Caller: refund unused deposit
```

## Deployment Hierarchy

The examples below use generic "Global Contract 1/2" names. In practice, these can be any global contracts — e.g. Escrow Swap, Oneshot Condvar, etc.

### How Global Contracts Work

[NEP-591](https://github.com/near/NEPs/blob/master/neps/nep-0591.md) introduces a protocol-level **Global Contract Namespace** — a mapping from identifiers to WASM contract code. Instead of each account storing its own copy of contract code, accounts reference global contracts via `UseGlobalContractAction`. Two deployment modes are supported:

- **Deploy-by-hash** (`GlobalContractDeployMode::CodeHash`): immutable — contract code is referenced by its SHA-256 hash. Cannot be changed after deployment.
- **Deploy-by-account-id** (`GlobalContractDeployMode::AccountId`): upgradeable — the owner can redeploy code. All references auto-update since they point to the account, not the hash.

### Bootstrap Process

1. Deploy GD globally **by code hash** (one-time, immutable)
2. Instantiate Controller with `StateInit` referencing GD's code hash → deterministic address
3. Controller calls `gd_approve` + `gd_deploy` of the same GD code under its own account ID
4. Controller is now a **mutable** GD instance (can upgrade GD itself)
5. Instantiate Global Contract 1 Controller referencing Controller's account ID + unique `code_hash` in `StateInit` (e.g. `0x...01`)
6. `gd_approve` + `gd_deploy` Global Contract 1 WASM on that instance
7. From Global Contract 1 Controller, create individual Global Contract 1 instances
8. Repeat for Global Contract 2 with a different `code_hash` in `StateInit` (e.g. `0x...02`)

### Hierarchy Diagram

```mermaid
%%{init: {"flowchart": {"wrappingWidth": 600}}}%%
flowchart TD
    GD["GLOBAL CONTRACT NAMESPACE<br/>CodeHash(0x123..123) => GLOBAL DEPLOYER WASM<br/>GlobalHash(0s..aaa) => GLOBAL DEPLOYER WASM<br/>GlobalAccountId(0s..bbb) => GLOBAL CONTRACT 1 WASM<br/>GlobalAccountId(0s..ccc) => GLOBAL CONTRACT 2 WASM"]

    C["MUTABLE CONTROLLER · 0s..aaa<br/>ref: CodeHash(0x123..123)<br/>state_init: {<br/>  owner: alice.near,<br/>  code_hash: 0x00..00,<br/>  approved_hash: 0x00..00<br/>}"]

    C --> GC1C["GLOBAL CONTRACT 1 CONTROLLER · 0s..bbb<br/>ref: GlobalAccountId(0s..aaa)<br/>state_init: {<br/>  owner: alice.near,<br/>  code_hash: 0x00..01,<br/>  approved_hash: 0x00..00<br/>}"]

    C --> GC2C["GLOBAL CONTRACT 2 CONTROLLER · 0s..ccc<br/>ref: GlobalAccountId(0s..aaa)<br/>state_init: {<br/>  owner: alice.near,<br/>  code_hash: 0x00..02,<br/>  approved_hash: 0x00..00<br/>}"]

    GC1C --> GC1I1["GLOBAL CONTRACT 1 · INSTANCE 1 · 0s..ddd<br/>ref: GlobalAccountId(0s..bbb)<br/>state_init: {<br/>  owner: alice.near,<br/>  code_hash: 0x00..01,<br/>  approved_hash: 0x00..00<br/>}"]

    GC1C --> GC1I2["GLOBAL CONTRACT 1 · INSTANCE 2 · 0s..eee<br/>ref: GlobalAccountId(0s..bbb)<br/>state_init: {<br/>  owner: alice.near,<br/>  code_hash: 0x00..02,<br/>  approved_hash: 0x00..00<br/>}"]

    GC2C --> GC2I1["GLOBAL CONTRACT 2 · INSTANCE 1 · 0s..fff<br/>ref: GlobalAccountId(0s..ccc)<br/>state_init: {<br/>  owner: alice.near,<br/>  code_hash: 0x00..01,<br/>  approved_hash: 0x00..00<br/>}"]

    style GD fill:#e0e0e0,stroke:#999,color:#000
    style C fill:#bbdefb,stroke:#1976d2,color:#000
    style GC1C fill:#c8e6c9,stroke:#388e3c,color:#000
    style GC2C fill:#ffe0b2,stroke:#f57c00,color:#000
    style GC1I1 fill:#c8e6c9,stroke:#388e3c,color:#000
    style GC1I2 fill:#c8e6c9,stroke:#388e3c,color:#000
    style GC2I1 fill:#ffe0b2,stroke:#f57c00,color:#000
```

### Deployed Instances

The Global Deployer WASM is built from the [global-deployer/v0.2.0](https://github.com/near/intents/releases/tag/global-deployer%2Fv0.2.0) release. The code hash is the same on both networks:

**Code hash:** `8JK2g3kr7qCbRDBmoLx7c9Zrz9TxPdANP7ocbQGE2fqP`

| Network | Type | Account / Hash | Tx |
|---------|------|----------------|-----|
| Mainnet | Immutable (by hash) | `8JK2g3kr7qCbRDBmoLx7c9Zrz9TxPdANP7ocbQGE2fqP` | [4RB52Rr...](https://nearblocks.io/txns/4RB52RrkSd8BVaAAUAV1okHWmDe2BRFm6tzva4HhY9Uy) |
| Mainnet | Mutable (by account ID) | [0s384bfa53f1718c7f53eaaa1b43c55e2aea3ef309](https://nearblocks.io/address/0s384bfa53f1718c7f53eaaa1b43c55e2aea3ef309) | [6o1ffgK...](https://nearblocks.io/txns/6o1ffgKv1fe6mLR2vtRDzMAdYJdunQRfzaLH5oZ8dwcz) |
| Testnet | Immutable (by hash) | `8JK2g3kr7qCbRDBmoLx7c9Zrz9TxPdANP7ocbQGE2fqP` | [HJrtRP1...](https://testnet.nearblocks.io/txns/HJrtRP1o3Hfv9Y42xUtECr2RqFafS1L3BM4J9Lgw4jTr) |
| Testnet | Mutable (by account ID) | [0s29e346108955b88c2d180a4ba17662b1f2cc1028](https://testnet.nearblocks.io/address/0s29e346108955b88c2d180a4ba17662b1f2cc1028) | [Bx2LLQV...](https://testnet.nearblocks.io/txns/Bx2LLQVrVyyGFc4natbJUGtotxSn1iJHe1GWNE5JnXCk) |

The mutable instances were created and deployed in a single transaction — `StateInit` pre-sets `approved_hash` to the GD code hash, so `gd_deploy` can be called immediately without owner action.

### Multi-Stage Deployment

If consecutive upgrades are needed (e.g. H1 → H2 → H3), they can be prepared upfront. Since `gd_approve` takes the current `code_hash` as `old_hash`, each approval call simply references the code hash of the previously approved WASM binary. As long as you know the hashes of all consecutive binaries in advance, the full chain of `gd_approve` + `gd_deploy` calls can be queued and executed sequentially.

### Important Notes

- A deterministic account ID is derived from `StateInit` at creation. After `gd_approve` mutates state, on-chain state diverges from what the address was derived from.
- Upgrading Controller code propagates to all future instances (deploy-by-account-id).
- The GD deployed by hash once is the **immutable foundation** for the whole hierarchy.

## `near gds` extension

The `near-gds` command is an extension for [near-cli-rs](https://github.com/near/near-cli-rs) that computes the `StateInit` for a global-deployer contract, outputting a JSON map of base64-encoded key-value pairs.

### Install

```sh
cargo install --path . --example near-gds
```

### Usage

```sh
$ near gds --help
Compute StateInit for a global-deployer contract

Usage: near-gds [OPTIONS] --owner-id <AccountId>

Options:
      --owner-id <AccountId>  Owner account ID
  -i, --index <N>             Unique index for the deployer instance. Can be used to derive
                              multiple deployers for a single owner [default: 0]
      --approve <HASH>        Pre-approve SHA-256 code hash: first `gd_deploy()` won't require
                              `gd_approve()`. Hash can be encoded as base58 or hex with `0x`
                              prefix
  -q, --quiet                 Output single-line JSON with base64-encoded keys/values
  -h, --help                  Print help
```

### Example

```bash
near gds --owner-id test.near --index 1
```
```
owner_id:       test.near
code_hash:      0000000000000000000000000000000000000000000000000000000000000001
approved_hash:  0000000000000000000000000000000000000000000000000000000000000000
{"":"CQAAAHRlc3QubmVhcgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="}
```

Use `-q` / `--quiet` to suppress stderr and emit only JSON, useful for piping into other tools like `near-cli` (supported after [near-cli-rs#560](https://github.com/near/near-cli-rs/pull/560) is released):

```bash
near contract state-init \
  use-global-account-id 0s384bfa53f1718c7f53eaaa1b43c55e2aea3ef309 \
  data-from-json "$(near gds \
    --owner-id intents.sputnik-dao.near --index 42 \
    --approve 0x6c71114931fe91153b868f2cb29c5db70e59677d6d2e40404b3b9044d8052266 \
    --quiet)"
```

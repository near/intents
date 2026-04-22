# Outlayer

Outlayer is a platform that enables verifiable off-chain computation.
It allows users to submit their execution requests from Near
smart-contracts (via [yield-resume](https://github.com/near/NEPs/blob/master/neps/nep-0519.md)) or via HTTPs API.

## TEE workers

Execution requests consist of WASM binary to execute and an input to it.
This request is submitted to a worker running inside of TEE environment,
which is capable of executing WASMs in its runtime.

## [Confidential Key Derivation (CKD)](https://github.com/near/mpc/blob/main/crates/threshold-signatures/docs/confidential_key_derivation/confidential-key-derivation.md)

During the boot process, a worker:

1. Generates ephemeral ed25519 keypair
2. Submits on-chain CKD request (along with ephemeral public key) to the
  [MPC contract](https://github.com/near/mpc/tree/main/crates/contract)
  via some proxy contract that verifies TEE attestation first.
3. Waits for CKD response from MPC network.
4. Descrypts the response via ephemeral private key generated on step 1.

This allows all workers that execute the same image to verify that they're
running inside of TEE and derive the same deterministic key (or seed).

## Host Functions

Host functions available to WASMs at runtime are specified in [./wit](./wit/) directory.

### Key Derivation and Signing

The root seed obtained via [CKD](#confidential-key-derivation-ckd) is
then used to derive secp256k1/ed25519 keypairs used for corresponding
`derive_public_key(path)` and `sign(path, msg)` host functions.

Each WASM is identified by `app_id` - AccountId of
[OutlayerApp](../../contracts/outlayer-app/README.md) contract.
Derivation paths from host-functions are implicitly prefixed with
`app_id` by the host implementation, so that different WASMs have
different derivation prefixes.

Note: WASMs do not have an access to private keys, they can only use host
functions to sign and derive keys.

Key derivation is non-hardened, which allows clients to derive all public
keys fully offline by knowing only the root public key for a specific
curve.
# NEAR Intents smart contracts

## What is NEAR Intents?

NEAR Intents is a smart contract developed for the NEAR blockchain. It facilitates atomic P2P transactions among peers, by allowing trustless transactions in the smart contract.

Please note that the main smart contract in the repository, under the directory `contracts/defuse`, is referred to as the "Verifier" in the ecosystem. Near Intents contains more components that work in tandem to achieve its purpose. Nevertheless, this smart contract, the Verifier, can be used independently without needing anything else.


### Example

Alice wants to trade 1000 USDT with Bob for 1000 USDC. If Alice sends her 1000 USDT first, she risks Bob not fulfilling the promise of sending his 1000 USDC. Same risk for Bob if he goes first.

Solution:

Both Alice and Bob create accounts in the [NEAR Intents smart contract](https://nearblocks.io/address/intents.near). They then deposit their 1000 USDT/USDC. They create two intents. In Alice's, Alice declares her will to lose 1000 USDT for 1000 USDC, and Bob creates another intent showing his will to lose 1000 USDC for 1000 USDT. Each of them sign their intent. They put both intents in an array, and then call [the function](https://near.github.io/intents/defuse/intents/trait.Intents.html#tymethod.execute_intents) `execute_intents` in the NEAR Intents smart contract with the intents' array.

The Verifier smart contract will evaluate the intents and check whether the requests can be fulfilled, and will ensure that the transaction is done atomically, and the 1000 USDC/USDT will be swapped.

Finally, Alice and Bob can withdraw their USDC/USDT from the Verifier smart contract to their individual accounts.

### Documentation

For more information on how to use the Intents ecosystem, please refer to [the documentation](https://docs.near-intents.org/).

For technical information about the Verifier smart contract programming primitives (and other smart contracts here), please refer to [the cargo documentation page](https://near.github.io/intents/).

### Payload traits

Intents bridges several external signing standards such as BIP-322, TIP-191,
and ERC-191. Each standard is represented by a small structure implementing the
[`Payload`](https://near.github.io/intents/defuse_crypto/payload/trait.Payload.html)
and [`SignedPayload`](https://near.github.io/intents/defuse_crypto/payload/trait.SignedPayload.html) traits.
These implementations allow the engine to hash and verify messages from the
different standards in a uniform way. They are used internally and are not part
of a stable public API.

### The name "defuse"

The name defuse is an old name for the smart contract that we use to execute intents. It is being phased out for NEAR Intents.

### Building and running

You can obtain a working copy of the smart contract and the ABI from [the releases page](https://github.com/near/intents/releases/).

Alternatively, enter the pinned development environment and build the smart contract yourself:

```shell
nix develop
```

Build smart contract separately:

```shell
make CONTRACT[/VARIANT] [REPRODUCIBLE=1]
```

Build all contracts at once:

```shell
make
```

NOTE: Reproducible build can be specified by setting `REPRODUCIBLE=1`

Run integration tests:

```shell
cargo integration-tests <defuse|poa|escrow-swap>
```

Or run all tests:

```shell
make test
```

For state migration testing set environmental var `DEFUSE_MIGRATE_FROM_LEGACY=1`
State migrations will be applied before all tests.
The tests will use data created prior to migration combined with newly created data to verify the integrity of the state.

Run clippy linter:

```shell
make clippy
```

After building, the artifacts of the build will be in the `res` directory.

### Release process

This repository uses [Release Please](https://github.com/googleapis/release-please) and Conventional Commits to maintain independent release lines for `defuse`, `global-deployer`, `poa-factory`, `poa-token`, and `wallet`. Their tags use the `<component>/v<version>` format, for example `defuse/v0.4.3`.

Every push to `main` creates or updates separate release PRs for affected components. Squash-merged PR titles should therefore follow the Conventional Commits format, such as `feat(defuse): add delegated authentication`. Merging a PR carrying the `autorelease: pending` label creates its tag and GitHub release, reproducibly builds the component artifacts, and uploads the signed assets.

Release Please evaluates commits within each component directory. A change confined to shared `crates/` must therefore include a corresponding change in every affected contract directory when those contracts need a release. Repository Actions settings must grant workflows read/write access and allow GitHub Actions to create pull requests; branch protection must allow the Release Please PR workflow to operate while retaining the normal required CI checks.

Each release includes the component's WASM files, optional ABI files, `SHA256SUMS`, and `SHA256SUMS.sigstore.json`. After downloading all assets, verify their GitHub Actions identity and checksums with:

```shell
cosign verify-blob \
  --bundle SHA256SUMS.sigstore.json \
  --certificate-oidc-issuer=https://token.actions.githubusercontent.com \
  --certificate-identity-regexp='^https://github.com/near/intents/.github/workflows/' \
  SHA256SUMS
sha256sum --check SHA256SUMS
```

The keyless Sigstore certificate and transparency-log proof bind the checksum manifest to this repository's GitHub Actions workflows. The checksum manifest then binds every uploaded contract artifact.

### Contracts in this repository

- Verifier/Defuse smart contract: The primary contract for NEAR Intents discussed in this readme file.
- `PoA Token` and `PoA factory` contract: Contracts responsible for the Proof of Authority bridge. These help in transferring tokens from other assets (e.g., Bitcoin, Ethereum, Solana, etc) to the NEAR blockchain, so that transactions in the NEAR Intents can happen.
- Controller interface: Interface [for contract](https://github.com/aurora-is-near/aurora-controller-factory) responsible for upgrading smart contracts and migrating their state.

# Near Intents

NEAR Intents is a NEAR smart contract suite that enables atomic peer-to-peer
intent execution. The primary contract (`contracts/defuse`, deployed at
`intents.near`) is referred to as the **Verifier**: it lets users sign
declarative "intents" (e.g. "I will trade 1000 USDT for 1000 USDC"), and
executes matching intents atomically. Around the Verifier sits a constellation
of supporting contracts (PoA bridge tokens, escrow swaps, deterministic
wallets, global-contract deployers) and off-chain runtimes (Outlayer).

## Environment setup (for an AI agent on a fresh sandbox)

This repo has a pinned Rust toolchain (see `rust-toolchain`: `1.86.0` with
`clippy`, `rustfmt`, target `wasm32-unknown-unknown`). On a fresh sandbox you
typically need to install:

1. **Rust toolchain via rustup** (auto-picks up `rust-toolchain` on first run):

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
     | sh -s -- -y --default-toolchain stable --profile minimal
   echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> /etc/sandbox-persistent.sh
   export PATH="$HOME/.cargo/bin:$PATH"
   ```

2. **C linker + libc headers** — `cargo build` invokes `cc` for proc-macro and
   build-script binaries:

   ```bash
   sudo apt-get update && sudo apt-get install -y gcc libc6-dev
   ```

3. **cargo-near** — required by `make` (the build pipeline shells out to
   `cargo near build non-reproducible-wasm`):

   ```bash
   cargo install cargo-near
   ```

4. **taplo-cli** — required by `make fmt` / `make check-fmt`:

   ```bash
   cargo install taplo-cli --locked
   ```

5. **cargo-machete** — required by `make check-unused-deps`:

   ```bash
   cargo install cargo-machete --locked
   ```

6. **jq** — the Makefile uses `jq` to parse `cargo metadata`:

   ```bash
   sudo apt-get install -y jq
   ```

Integration tests under `tests/` boot a NEAR sandbox node via the `near-sandbox`
crate, which downloads a `neard` binary for the host platform on first run; no
manual install needed.

### Do NOT run `cargo build` at the workspace root

`near-sdk` emits a `compile_error!` when it is built for a non-wasm target
without one of its narrow allow-list of cfgs. The supported entry points are
`make` (which uses `cargo near build`) and `cargo check --target
wasm32-unknown-unknown` for compile-checking contracts. Use `cargo test
--workspace` for the non-contract crates — that path is fine because the
`near-sdk` consumers there are gated on `cfg(test)` or `feature = "unit-testing"`.

## Build / test / lint commands

Everything is wired through the top-level `Makefile`. Key targets:

### Builds (produce `.wasm` + `.abi.json` in `res/`)

| Command                                  | Effect                                                                    |
| ---------------------------------------- | ------------------------------------------------------------------------- |
| `make` / `make all`                      | Build **every** contract listed in `CONTRACT_CRATES` (see below).         |
| `make <contract>/all`                    | Build a single contract, including all variants (e.g. `make defuse/all`). |
| `make <contract>`                        | Build the default variant only (e.g. `make defuse`).                      |
| `make <contract>/<variant>`              | Build a specific variant (e.g. `make defuse/far`).                        |
| `make REPRODUCIBLE=1 ...`                | Use `cargo near build reproducible-wasm` (requires Docker/Podman).        |

Build artifacts land in `res/` as `<crate>.wasm` (+ optional `<crate>.<variant>.wasm`)
plus matching `.abi.json` files. The Makefile generates targets dynamically from
`cargo metadata` filtered by `CONTRACT_CRATES`.

### Tests

| Command                                                    | Effect                                                                                     |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `make test`                                                | `cargo test --workspace --all-targets`. Includes integration tests under `tests/`.         |
| `cargo integration-tests <feature>`                        | Alias for `cargo test -p defuse-tests --no-default-features --features <feature>`.         |
| `DEFUSE_MIGRATE_FROM_LEGACY=1 make test`                   | Run state-migration tests against `releases/previous.wasm`.                                |

Valid `<feature>` values for `cargo integration-tests`: `defuse`, `poa`,
`escrow-swap`, `wallet`, `outlayer`, `deployer`, `long` (see `tests/Cargo.toml`).

#### Tests depend on built wasm — **rebuild before re-running**

Integration tests load contract wasm from `res/` via
`crates/testing/utils/src/wasms.rs` (uses the env var `DEFUSE_USE_OUT_DIR=res`,
set in `.cargo/config.toml`). This means:

- **First run after a clean checkout:** run `make` (or at minimum `make
  <contract>/all` for the contracts your tests touch) **before** `make test`,
  otherwise the test will panic on `Failed to read WASM file at res/...`.
- **After modifying a contract** (or any crate it depends on transitively),
  rebuild that contract's wasm before re-running the affected test —
  otherwise the test executes stale bytecode and you will chase phantom bugs.
  The fastest path is `make <contract>` (default variant only).
- A small set of wasm artifacts is checked into `releases/` (`previous.wasm`,
  `wnear.wasm`, `non-fungible-token.wasm`) and read directly without a build
  step.

### Lints / formatting / checks

| Command                  | Effect                                                                                       |
| ------------------------ | -------------------------------------------------------------------------------------------- |
| `make check`             | `make check-contracts` + `cargo clippy --workspace --all-targets --no-deps`.                 |
| `make check-contracts`   | Per-contract `cargo clippy -p <crate> --no-deps --target wasm32-unknown-unknown`.            |
| `make check-fmt`         | `cargo fmt --all --check` + `taplo format --check`.                                          |
| `make check-unused-deps` | `cargo machete`.                                                                             |
| `make check-all`         | `check-fmt` + `check-unused-deps` + `check`.                                                 |
| `make fmt`               | `cargo fmt --all` + `taplo format` — apply formatting in place.                              |
| `make clean`             | `rm -rf res/ && cargo clean`.                                                                |
| `make help`              | Print discovered targets.                                                                    |

## Repository layout

Top-level dirs:

- `contracts/` — on-chain contracts, one subdirectory per contract.
- `crates/` — reusable libraries used by contracts and by the off-chain stack.
  Grouped by area: `near/`, `primitives/`, `signatures/`, `wallet/`,
  `testing/`, `outlayer/`, plus a few flat utility crates (`bitmap`,
  `borsh-utils`, `crypto`, `io-utils`, `map-utils`, `num-utils`,
  `serde-utils`).
- `tests/` — integration test crate (`defuse-tests`). Loads built wasm from
  `res/` and drives a sandbox `neard` via `near-sandbox`. Test stubs live
  under `tests/contracts/`.
- `res/` — build output (gitignored). Populated by `make`.
- `releases/` — pinned wasm binaries committed for migration & cross-contract
  tests (`previous.wasm` = last published Verifier, `wnear.wasm`,
  `non-fungible-token.wasm`).
- `scripts/` — release tooling.
- `rust-toolchain`, `Cargo.toml`, `Makefile`, `taplo.toml`, `.cargo/config.toml`
  — workspace configuration. `.cargo/config.toml` defines the
  `integration-tests` alias and sets `DEFUSE_USE_OUT_DIR=res`.

## Subsystems

Per-area notes live next to the code. Open the relevant file when you start
working in that subtree:

- `contracts/defuse/CLAUDE.md` — the **Verifier**, the core of NEAR Intents
  (largest contract in the repo, `far` variant available).
- `contracts/escrow-swap/CLAUDE.md` — deterministic escrow-swap (NEP-616,
  immutable params).
- `contracts/global-deployer/CLAUDE.md` — two-step approve/deploy manager for
  global contract code on NEP-616 accounts (upgrade mechanism for NEP-591).
- `contracts/outlayer-app/CLAUDE.md` — on-chain piece of Outlayer; per-app
  `(code_hash, code_url)` reference at a deterministic account.
- `contracts/poa/CLAUDE.md` — Proof-of-Authority bridge factory + token.
- `contracts/wallet/CLAUDE.md` — generic sharded account-abstraction wallet
  (**not audited yet**).
- `crates/outlayer/CLAUDE.md` — the off-chain Outlayer runtime
  (wasmtime-based component sandbox), spanning 9 subcrates. This is becoming a
  load-bearing piece of the architecture — read it before touching anything
  under `crates/outlayer/`.

Test-only contracts:

- `tests/contracts/multi-token-receiver-stub` — minimal NEP-245 receiver.

## Terminology

### [nearcore](https://github.com/near/nearcore)

Reference implementation of the NEAR Protocol, written in Rust. Contains the
blockchain node (client), runtime, chain, and networking layers. Validators run
nearcore to produce and validate blocks. Smart contracts execute in a WASM VM
within the runtime.

### [Chain Signatures](https://github.com/near/mpc)

Enables NEAR smart contracts to sign transactions on external chains via a
distributed MPC network. Supports ECDSA (secp256k1) and EdDSA (Ed25519)
threshold signatures with FROST-based DKG. A NEAR indexer monitors the signer
contract and routes signing requests to MPC nodes; the leader submits the
final signature back on-chain.

### [Omni-Bridge](https://github.com/Near-One/omni-bridge)

Multi-chain asset bridge for transferring tokens between NEAR and other chains
(Ethereum, Bitcoin, Solana, Arbitrum, Base, Polygon, BNB, Zcash). NEAR→other
uses Chain Signatures (MPC-based signing); other→NEAR uses light clients or
Wormhole verification. Consists of bridge contracts on each chain, proof
verifiers, token factories, and Rust/JS SDKs.

### [NEPs: Near Enhancement Proposal](https://github.com/near/NEPs)

Near Enhancement Proposals (NEPs) are protocol specifications and standards.
The notably important NEPs for this project are:

#### [NEP-448: Zero Balance Accounts (ZBAs)](https://github.com/near/NEPs/blob/master/neps/nep-0448.md)

Accounts with ≤770 bytes storage are exempt from storage staking.
Creation cost is absorbed into the transaction fee, enabling onboarding
without acquiring NEAR tokens first.

#### [NEP-519: yield/resume promises](https://github.com/near/NEPs/blob/master/neps/nep-0519.md)

Allows contracts to defer a callback via `promise_yield_create` (returns a
resumption token) and later trigger it with `promise_yield_resume` (passes
payload data). Times out after 200 blocks if not resumed. Enables async
request-response patterns, e.g. waiting for MPC signature computation.

#### [NEP-591: Global Contracts](https://github.com/near/NEPs/blob/master/neps/nep-0591.md)

Deploy contract code once, reference it from multiple accounts via
`DeployGlobalContractAction`/`UseGlobalContractAction`. Two modes: by CodeHash
(immutable) or AccountId (updatable). Code replicates across all shards.
Deployment burns at 10× storage rate; usage cost is based on identifier length.

#### [NEP-616: Deterministic AccountIds](https://github.com/near/NEPs/blob/master/neps/nep-0616.md)

Account IDs derived as `"0s" .. hex(keccak256(borsh(StateInit))[12..32])`,
where `StateInit` = global contract ID + initial storage. Enables sharded
contract designs atop global contracts (NEP-591). New `StateInit` action
deploys/initializes at the derived ID. Anyone can deploy; only the contract
itself can mutate state. Supports `refund_to` for custom refund routing.

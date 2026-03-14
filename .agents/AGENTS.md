# Near Intents

TODO

## Project structure

TODO

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


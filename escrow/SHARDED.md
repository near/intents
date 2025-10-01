
# Sharded Escrow smart-contracts

Escrow is a contract that locks funds under specific terms and unlocks them when these conditions were met.

## Why not "Vaults"?

Logic of supported conditions is the core logic of escrow smart-contract. So, if another external contract is responsible for this logic, then it makes no sense to introduce this intermediary "escrow" contract that only forwards `.*_transfer()` Promises from a single whitelisted "owner".  
This logic is already implemented by FT/sFT contracts.

## Why sharded (i.e. deterministic per-escrow) contracts?

### Pros:

* Sharding == more throughput

* ZBAs let us avoid `.storage_deposit()` if we fit into these limits.
  Storage Management is a pain.

  Moreover, If a user doesn't have NEAR, then he needs first to buy it.  
  But what if he trades some instantly-illiquid tokens and can't sell small portion of it right away? What about NFTs?

* Ecosystem projects will have a propper primitive instead of building on top of legacy approaches.

### Cons:
* Slower settlement?  
  We do XCCs anyway, so few more blocks for async interactions is ok IMO

## Authentication

User authentication is ideally permofmed by per-user wallet-contracts with Deterministic AccountIds from NEP-616.
In short-term we can use `intents.near` for that.

## RFQ process:

```mermaid
---
title: RFQ
---
sequenceDiagram
    Actor user as alice.near
    Participant solverBus as Solver Bus
    Actor solver as solver.near

    user ->> solverBus: RFQ: 100 A -> ??? B
    solverBus ->> solver: RFQ: 100 A -> ??? B
    solver ->> solverBus: Quote: 100 A -> 200 B (solver.near)
    solverBus ->>user: Quote: 100 A -> 200 B: [solver.near, ...]
```

## Escrow Params

* Maker Asset
* Maker Amount: fixed

* Taker Asset (or `.custom_resolve()` for pizza case)
* Taker Amount
  * Fixed: (optional) + slippage
  * Dutch Auction: price points
  * Market Orders: via oracle-verified price within slippage
* Allow partial fills?
* Deadline
* Solvers whitelist (optional)
* `receiver_id` for Taker Asset
* Fee receiver

### Escrow Actions

* Open
* (Partial) Fill
* Increase Maker Amount / Decrease Taker Amount
* Cancel: after `deadline` passed 


## Open Escrow

Here is a full flow with sharded FTs.
The same can be achieved via non-sharded FTs, but it would require sending separate `.state_init()`, which complicates things a bit.

```mermaid
---
title: Open Escrow
---
sequenceDiagram
    Actor user as alice.near
    Participant user_sft_wallet_a as User sFT wallet A (0s123...)
    Participant escrow_sft_wallet_a as Escrow sFT wallet A (0s...)
    Participant escrow as Escrow (0s...)

    user ->>+ user_sft_wallet_a: .sft_transfer()
    user_sft_wallet_a ->>+ escrow_sft_wallet_a: .state_init()<br/>.sft_receive()
    escrow_sft_wallet_a ->> escrow: .state_init()<br/>.sft_on_receive()
    Note over escrow: Open with params
    escrow_sft_wallet_a ->>- escrow_sft_wallet_a: .sft_resolve()
    user_sft_wallet_a ->>- user_sft_wallet_a: .sft_resolve()
```

### Fill Escrow

```mermaid
---
title: Fill Escrow
---
sequenceDiagram
    Actor solver as solver.near
    Participant solver_sft_wallet_b as Solver sFT wallet B (0s123...)
    Participant escrow_sft_wallet_b as Escrow sFT wallet B (0s...)
    Participant escrow as Escrow (0s...)
    Participant escrow_sft_wallet_a as Escrow sFT wallet A (0s...)
    Participant user_sft_wallet_b as User sFT wallet B (0s...)
    Participant solver_sft_wallet_a as Solver sFT wallet A (0s...)


    solver ->>+ solver_sft_wallet_b: .sft_transfer()
    solver_sft_wallet_b ->>+ escrow_sft_wallet_b: .state_init()<br/>.sft_receive()
    escrow_sft_wallet_b ->>+ escrow: .sft_on_receive()
    Note over escrow: Fill
    par Send to Solver
      escrow ->>+ escrow_sft_wallet_a: .sft_transfer()
      escrow_sft_wallet_a ->> solver_sft_wallet_a: .sft_receive()
      escrow_sft_wallet_a ->>- escrow_sft_wallet_a: .sft_resolve()
    and Send to User
      escrow ->> escrow_sft_wallet_b: .sft_transfer()
      escrow_sft_wallet_b ->> user_sft_wallet_b: .sft_receive()
      escrow_sft_wallet_b ->> escrow_sft_wallet_b: .sft_resolve()
    end
    deactivate escrow
    escrow_sft_wallet_b ->>- escrow_sft_wallet_b: .sft_resolve()
    solver_sft_wallet_b ->>- solver_sft_wallet_b: .sft_resolve()
```

### Partial fills (via safety deposit)

#### Lock

```mermaid
---
title: Lock for Partial fill
---
sequenceDiagram
    Actor solver as solver1.near
    Participant escrow as Escrow (0s...)
    Participant escrow_sft_wallet_a as Escrow sFT wallet A (0s...)
    Participant sub_escrow_sft_wallet_a as Sub-Escrow sFT wallet A (0s...)
    Participant sub_escrow as Sub-Escrow (0s...)

    solver ->>+ escrow: .partial_escrow(amount)<br/> + attach_deposit(safety_deposit)
    escrow ->>+ escrow_sft_wallet_a: .sft_transfer()<br/> + attach_deposit(safety_deposit)
    escrow_sft_wallet_a ->>+ sub_escrow_sft_wallet_a: .state_init()<br/>.sft_receive()<br/> + attach_deposit(safety_deposit)
    sub_escrow_sft_wallet_a ->> sub_escrow: .state_init()<br/>.sft_on_receive()<br/> + attach_deposit(safety_deposit)
    Note over sub_escrow: Open with params<br/> + Safety deposit
    sub_escrow_sft_wallet_a ->>- sub_escrow_sft_wallet_a: .sft_resolve()
    escrow_sft_wallet_a ->>- escrow_sft_wallet_a: .sft_resolve()
    escrow ->>- escrow: .resolve_partial_escrow()
```

#### Fill

Same as non-partial fills, but also refund `safety_deposit` to `solver1.near`.

### What we need
* NEP for sFT standard
  > Reference implementation already exists
* Indexing of all global contracts usages
  > Should be relatively easy, already discussed with @Anton Astafiev
* Indexing for all sFT mints/transfers/burns
* Indexing for all escrow contracts


### One Click Swap as Escrow

Why to even bother with implementing temporary non-sharded smart-contracts if we can live for now with escrow implemented as EOAs via 1CS?
#[cfg(not(any(feature = "nep141", feature = "nep245")))]
compile_error!(
    r#"At least one of these features should be enabled:
- "nep141"
- "nep245"
"#
);

#[cfg(feature = "contract")]
mod contract;

pub mod action;
#[cfg(feature = "auth_call")]
pub mod auth_call;
mod error;
pub mod event;
mod state;
mod utils;

pub use self::{error::*, state::*};

pub use defuse_deadline::Deadline;
pub use defuse_decimal as decimal;
pub use defuse_fees::Pips;
pub use defuse_token_id as token_id;

use near_sdk::{PromiseOrValue, ext_contract};

#[ext_contract(ext_escrow)]
pub trait Escrow {
    fn es_view(&self) -> &Storage;

    /// Closes the escrow + performs escrow_lost_found().
    ///
    /// It's allowed to close when:
    /// * Deadline has expired (permissionless)
    /// * maker_src_remaining == 0 && predecessor == maker
    /// * taker_whitelist == [predecessor]
    fn es_close(&mut self, params: Params) -> PromiseOrValue<bool>;

    /// Retries sending:
    /// * `maker_src_remaining` if the escrow was closed
    /// * `maker_dst_lost` if any
    ///
    /// Returns whether contract was fully cleaned up, so you can
    /// stop indexing it.
    /// Otherwise, there MIGHT be lost assets there
    /// or they might come in the future.
    // TODO: maker custom params for withdrawal
    fn es_lost_found(&mut self, params: Params) -> PromiseOrValue<bool>;
}

// TODO: add support for NFTs
// TODO: recover()
// TODO: total_fee(&self)
// TODO: effective_price(&self)
// TODO: solver: create_subescrow and lock NEAR on it
// TODO: refund locked NEAR back to taker if closed Ok, otherwise...?
// TODO: add support for custom ".on_closed()" hooks?
// TODO: cross-escrow fills without the need for liquidity from solvers
// TODO: coinsidence of wants?
// user1: locked 1 BTC in escrow for swap to 100k USDC
// user2: sends RFQ to SolverBus to swap 10k USDC to BTC
// SolverBus sends him address of escrow contract,
// user2 signs "transfer" intent:
// `{
//   "receiver_id": "0s123...abc" // address of escrow
//   "token": "<USDC ADDRESS>",
//   "amount": "10k",
//   "msg": "FILL MSG + SOLVER_BUS SIGNATURE",
// }`
// user2 transfers to "solver-bus-proxy.near" escrow, tries to fill, if fail -> refund
// OR: we can have intermediary contract to refund to ANOTHER ESCROW to reduce failure rate
//
// if we make solvers to be MMs, then solver-bus-proxy.near can
// implement CLOB

// TODO: lending
// solver -> escrow::mt_on_transfer(sender_id, token_id, amount, msg)
//        * msg: loan
//
//        -> escrow_loan:
//

#[cfg(not(any(feature = "nep141", feature = "nep245")))]
compile_error!(
    r#"At least one of these features should be enabled:
- "nep141"
- "nep245"
"#
);

#[cfg(feature = "contract")]
mod contract;

#[cfg(feature = "auth_call")]
pub mod auth_call;
mod error;
mod event;
mod price;
pub mod state;
pub mod tokens;
mod utils;

pub use self::{error::*, event::*, price::*};

pub use defuse_fees::Pips;
pub use defuse_near_utils::time::Deadline;
pub use defuse_token_id::TokenId;

use crate::state::{Params, Storage};

// TODO: more pub re-exports

use near_sdk::{PromiseOrValue, ext_contract};

#[ext_contract(ext_escrow)]
pub trait Escrow {
    fn escrow_view(&self) -> &Storage;

    /// Closes the escrow + performs lost_found().
    ///
    /// It's allowed to close when:
    /// * Deadline has expired (permissionless)
    /// * maker_src_remaining == 0 && predecessor == maker
    /// * taker_whitelist == [predecessor]
    ///
    /// If deadline has not exceeded yet, then fails.
    /// Returns whether was closed just now, or false if was already closed.
    fn escrow_close(&mut self, params: Params) -> PromiseOrValue<bool>;

    /// Retries sending:
    /// * `maker_src_remaining` if the escrow was closed
    /// * `maker_dst_lost` if any
    ///
    /// Returns whether contract was fully cleaned up, so you can
    /// stop indexing it.
    /// Otherwise, there MIGHT be lost assets there
    /// or they might come in the future.
    /// TODO: maker custom params for withdrawal
    fn escrow_lost_found(&mut self, params: Params) -> PromiseOrValue<bool>;
    // TODO: recover()
    // TODO: decrease_price()
    // TODO: prolongate_deadline()
    // TODO: total_fee(&self)
    // TODO: effective_price(&self)
    // TODO: create sub_escrow for a single solver and lock NEAR
}

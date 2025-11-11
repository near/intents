mod action;
#[cfg(feature = "contract")]
pub mod contract;
mod error;
mod event;
mod price;
mod state;
mod utils;

pub use self::{action::*, error::*, event::*, price::*, state::*};

pub use defuse_near_utils::time::Deadline;

// TODO: more pub re-exports

use near_sdk::{AccountId, Gas, PromiseOrValue, ext_contract, near};

// TODO: create sub_escrow for a single solver and lock NEAR

#[ext_contract(ext_escrow)]
pub trait Escrow {
    fn view(&self) -> &Storage;

    /// Closes the escrow + performs lost_found().
    ///
    /// It's allowed to close when:
    /// * Deadline has expired (permissionless)
    /// * maker_src_remaining == 0 && predecessor == maker
    /// * taker_whitelist == [predecessor]
    ///
    /// If deadline has not exceeded yet, then fails.
    /// Returns whether was closed just now, or false if was already closed.
    fn close(&mut self, fixed_params: FixedParams) -> PromiseOrValue<bool>;

    /// Retries sending:
    /// * `maker_src_remaining` if the escrow was closed
    /// * `maker_dst_lost` if any
    ///
    /// Returns whether contract was fully cleaned up, so you can
    /// stop indexing it.
    /// Otherwise, there MIGHT be lost assets there
    /// or they might come in the future.
    fn lost_found(&mut self, fixed_params: FixedParams) -> PromiseOrValue<bool>;
    // TODO: decrease_price()
    // TODO: prolongate_deadline()
    // TODO: total_fee(&self)
    // TODO: effective_price(&self)
}

// TODO: notify on_close()
// TODO: on_auth

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SendParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receiver_id: Option<AccountId>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_gas: Option<Gas>,
}

impl SendParams {
    pub fn verify(&self) -> Result<()> {
        // TODO: verify min_gas < MAX_GAS
        Ok(())
    }

    pub const fn is_call(&self) -> bool {
        self.msg.is_some()
    }
}

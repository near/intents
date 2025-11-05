mod action;
#[cfg(feature = "contract")]
pub mod contract;
mod error;
mod event;
mod price;
mod state;
mod utils;

pub use self::{action::*, error::*, event::*, price::*, state::*};

use defuse_nep245::receiver::MultiTokenReceiver;
use near_sdk::{AccountId, Gas, PromiseOrValue, ext_contract, json_types::U128, near};

#[ext_contract(ext_escrow)]
pub trait Escrow: MultiTokenReceiver {
    fn view(&self) -> &Storage;
    fn close(&mut self, fixed_params: FixedParams) -> PromiseOrValue<U128>;

    // TODO: total_fee()
    // TODO: effective_price()
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

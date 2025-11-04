mod action;
#[cfg(feature = "contract")]
pub mod contract;
mod error;
mod event;
mod price;
mod state;

pub use self::{action::*, error::*, event::*, price::*, state::*};

use defuse_nep245::receiver::MultiTokenReceiver;
use near_sdk::{PromiseOrValue, ext_contract, json_types::U128};

#[ext_contract(ext_escrow)]
pub trait Escrow: MultiTokenReceiver {
    fn view(&self) -> &Storage;
    fn close(&mut self, fixed_params: FixedParams) -> PromiseOrValue<U128>;

    // TODO: total_fee()
    // TODO: effective_price()
}

// TODO: notify on_close()

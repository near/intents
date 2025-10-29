mod action;
#[cfg(feature = "contract")]
mod contract;
mod error;
mod event;
mod price;
mod state;

pub use self::{action::*, error::*, event::*, price::*, state::*};

use defuse_nep245::receiver::MultiTokenReceiver;
use near_sdk::{Promise, ext_contract};

#[ext_contract(ext_escrow)]
pub trait Escrow: MultiTokenReceiver {
    fn view(&self) -> &Storage;
    fn close(&mut self) -> Promise;

    // TODO: total_fee()
    // TODO: effective_price()
}

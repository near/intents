#[cfg(feature = "auth_call")]
mod auth_call;
mod cleanup;
mod close;
mod fill;
mod fund;
mod lost_found;
mod resolve;
mod return_value;
mod tokens;

use defuse_near_utils::UnwrapOrPanic;
use impl_tools::autoimpl;
use near_sdk::{PanicOnDefault, PromiseOrValue, env, near};

use crate::{ContractStorage, Error, Escrow, Params, Result, Storage};

#[near(contract_state(key = ContractStorage::STATE_KEY))]
#[autoimpl(Deref using self.0)]
#[derive(Debug, PanicOnDefault)]
pub struct Contract(ContractStorage);

#[near]
impl Escrow for Contract {
    fn escrow_view(&self) -> &Storage {
        self.try_as_alive()
            // if cleanup is in progress, the contract will be
            // soon deleted anyway, so it's ok to panic here
            .unwrap_or_panic()
    }

    fn escrow_close(&mut self, params: Params) -> PromiseOrValue<bool> {
        self.close(&env::predecessor_account_id(), params)
            .unwrap_or_panic()
    }

    fn escrow_lost_found(&mut self, params: Params) -> PromiseOrValue<bool> {
        self.lost_found(params).unwrap_or_panic()
    }
}

impl ContractStorage {
    #[inline]
    const fn as_alive(&self) -> Option<&Storage> {
        self.0.as_ref()
    }

    #[inline]
    fn try_as_alive(&self) -> Result<&Storage> {
        self.as_alive().ok_or(Error::CleanupInProgress)
    }
}

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

use std::borrow::Cow;

use defuse_near_utils::UnwrapOrPanic;
use near_sdk::{FunctionError, PanicOnDefault, PromiseOrValue, env, near, require};

use crate::{
    Error, Escrow, Params, Storage,
    event::{EscrowIntentEmit, Event},
};

#[near(contract_state)] // TODO: (key = "")
#[derive(Debug, PanicOnDefault)]
pub struct Contract(Option<Storage>);

#[near]
impl Contract {
    #[init]
    pub fn escrow_init(params: &Params) -> Self {
        Event::Created(Cow::Borrowed(&params)).emit();

        if params.deadline.has_expired() {
            Error::DeadlineExpired.panic();
        }
        let s = Storage::new(params).unwrap_or_panic();

        // just for the safety
        require!(
            env::current_account_id() == s.derive_account_id(env::predecessor_account_id()),
            "wrong params or factory"
        );

        Self(Some(s))
    }
}

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

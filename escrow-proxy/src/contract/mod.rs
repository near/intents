mod admin;
mod tokens;
mod upgrade;
mod utils;

use defuse_escrow_swap::ext_escrow;
use defuse_oneshot_condvar::{CondVarContext, ext_oneshot_condvar};
use near_sdk::{
    AccountId, CryptoHash, Gas, NearToken, PanicOnDefault, Promise, env, near, require,
    state_init::{StateInit, StateInitV1},
};

use crate::EscrowProxy;
use crate::message::EscrowParams;
use crate::state::ProxyConfig;

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    config: ProxyConfig,
}

#[near]
impl Contract {
    #[init]
    #[must_use]
    #[allow(clippy::use_self)]
    pub fn new(config: ProxyConfig) -> Contract {
        Self { config }
    }

    fn assert_owner(&self) {
        require!(
            env::predecessor_account_id() == self.config.owner,
            "Only owner can call this method"
        );
    }
}

#[near]
impl EscrowProxy for Contract {
    fn config(&self) -> &ProxyConfig {
        &self.config
    }

    fn context_hash(&self, context: CondVarContext<'static>) -> CryptoHash {
        context.hash()
    }
}

#[near]
impl Contract {
    pub fn cancel_escrow(&self, params: EscrowParams) -> Promise {
        self.assert_owner();
        let escrow_address = {
            let raw_state = defuse_escrow_swap::ContractStorage::init_state(&params)
                .unwrap_or_else(|e| env::panic_str(&format!("Invalid escrow params: {e}")));
            let state_init = StateInit::V1(StateInitV1 {
                code: self.config.escrow_swap_contract_id.clone(),
                data: raw_state,
            });
            state_init.derive_account_id()
        };
        ext_escrow::ext(escrow_address)
            .with_static_gas(Gas::from_tgas(50))
            .es_close(params)
    }

    pub fn authorize(&self, condvar_id: AccountId) -> Promise {
        self.assert_owner();
        ext_oneshot_condvar::ext(condvar_id)
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(Gas::from_tgas(50))
            .cv_notify_one()
    }
}

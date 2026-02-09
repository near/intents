#[cfg(all(feature = "auth-call", feature = "escrow-swap"))]
mod auth_call;
mod tokens;
mod utils;

use crate::CondVarContext;
#[cfg(feature = "escrow-swap")]
use defuse_escrow_swap::{Params as EscrowParams, ext_escrow};
use near_sdk::{
    AccountId, CryptoHash, Gas, PanicOnDefault, Promise, env, json_types::U128, near, require,
};

use crate::EscrowProxy;
use crate::state::{ContractStorage, ProxyConfig};

#[near(contract_state(key = ContractStorage::STATE_KEY))]
#[derive(PanicOnDefault)]
pub struct Contract(ContractStorage);

#[near]
impl Contract {
    fn assert_owner(&self) {
        require!(
            env::predecessor_account_id() == self.0.config().owner_id,
            "only owner can call this method"
        );
    }
}

#[near]
impl EscrowProxy for Contract {
    /// Returns proxy configuration
    fn config(&self) -> &ProxyConfig {
        self.0.config()
    }

    /// Calculates oneshot condvar context hash that is required to derive condvar contract
    /// instance address
    fn context_hash(&self, context: CondVarContext<'static>) -> CryptoHash {
        context.hash()
    }

    /// Calculates oneshot condvar contract instance address, helper function for integration
    /// purposes, and easy calculation of oneshot condvar contract instance address in case of
    /// need for direct authorization using OneshotCondvar::cv_notify_one
    /// sender_id: The account id of the sender
    /// token_ids: The token ids of the tokens being transferred
    /// amounts: The amounts of the tokens being transferred
    /// msg: escrow proxy transfer message
    fn ep_approve_account_id(
        &self,
        sender_id: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> AccountId {
        use std::borrow::Cow;
        let context_hash = CondVarContext {
            sender_id: Cow::Owned(sender_id),
            token_ids: Cow::Owned(token_ids),
            amounts: Cow::Owned(amounts),
            msg: Cow::Borrowed(&msg),
        }
        .hash();
        let auth_contract_state_init = self.transfer_auth_state_init(context_hash);
        auth_contract_state_init.derive_account_id()
    }
}

#[cfg(feature = "escrow-swap")]
#[near]
impl Contract {
    pub fn es_cancel(&self, contract_id: AccountId, params: EscrowParams) -> Promise {
        self.assert_owner();
        ext_escrow::ext(contract_id)
            .with_static_gas(Gas::from_tgas(50))
            .es_close(params)
    }
}

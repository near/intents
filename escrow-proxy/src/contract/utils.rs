use std::borrow::Cow;

use crate::CondVarContext;
use defuse_oneshot_condvar::{
    CV_WAIT_GAS, ext_oneshot_condvar,
    storage::{Config as CondVarConfig, ContractStorage},
};
use near_sdk::{
    AccountId, NearToken, Promise, env,
    json_types::U128,
    state_init::{StateInit, StateInitV1},
};

use super::Contract;

impl Contract {
    pub(crate) fn transfer_auth_state_init(&self, salt: [u8; 32]) -> StateInit {
        let config = self.0.config();
        let state = CondVarConfig {
            #[cfg(feature = "auth-call")]
            on_auth_caller: config.on_auth_caller.clone(),
            notifier_id: config.notifier_id.clone(),
            waiter: env::current_account_id(),
            salt,
        };

        StateInit::V1(StateInitV1 {
            code: config.oneshot_condvar_global_id.clone(),
            data: ContractStorage::init_state(state).unwrap(),
        })
    }

    /// Creates the authorization promise for token transfers.
    /// Returns the cv_wait Promise.
    pub(crate) fn wait_for_authorization(
        &self,
        sender_id: &AccountId,
        token_ids: &[defuse_nep245::TokenId],
        amounts: &[U128],
        msg: &str,
    ) -> Promise {
        let context_hash = CondVarContext {
            sender_id: Cow::Borrowed(sender_id),
            token_ids: Cow::Borrowed(token_ids),
            amounts: Cow::Borrowed(amounts),
            msg: Cow::Borrowed(msg),
        }
        .hash();

        let auth_contract_state_init = self.transfer_auth_state_init(context_hash);
        let auth_contract_id = auth_contract_state_init.derive_account_id();

        ext_oneshot_condvar::ext_on(
            Promise::new(auth_contract_id)
                .state_init(auth_contract_state_init, NearToken::from_near(0)),
        )
        .with_static_gas(CV_WAIT_GAS)
        .with_unused_gas_weight(0)
        .cv_wait()
    }
}

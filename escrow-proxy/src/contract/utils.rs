use std::borrow::Cow;

use defuse_oneshot_condvar::{
    CondVarContext, WAIT_GAS, ext_oneshot_condvar,
    storage::{ContractStorage, StateInit as CondVarStateInit},
};
use near_sdk::{
    AccountId, NearToken, Promise, PromiseResult, env,
    json_types::U128,
    serde_json,
    state_init::{StateInit, StateInitV1},
};

use super::Contract;

impl Contract {
    pub(crate) fn get_deterministic_transfer_auth_state_init(
        &self,
        msg_hash: [u8; 32],
    ) -> StateInit {
        let state = CondVarStateInit {
            escrow_contract_id: self.config.escrow_swap_contract_id.clone(),
            auth_contract: self.config.auth_contract.clone(),
            on_auth_signer: self.config.auth_collee.clone(),
            authorizee: env::current_account_id(),
            msg_hash,
        };

        StateInit::V1(StateInitV1 {
            code: self.config.per_fill_contract_id.clone(),
            //TODO: get rid of unwrap
            data: ContractStorage::init_state(state).unwrap(),
        })
    }

    /// Creates the authorization promise for token transfers.
    /// Returns the cv_wait Promise.
    pub(crate) fn create_cv_wait_cross_contract_call(
        &self,
        sender_id: &AccountId,
        token_ids: &[defuse_nep245::TokenId],
        amounts: &[U128],
        salt: [u8; 32],
        msg: &str,
    ) -> Promise {
        let context_hash = CondVarContext {
            sender_id: Cow::Borrowed(sender_id),
            token_ids: Cow::Borrowed(token_ids),
            amounts: Cow::Borrowed(amounts),
            salt,
            msg: Cow::Borrowed(msg),
        }
        .hash();

        let auth_contract_state_init =
            self.get_deterministic_transfer_auth_state_init(context_hash);
        let auth_contract_id = auth_contract_state_init.derive_account_id();
        let auth_call = Promise::new(auth_contract_id)
            .state_init(auth_contract_state_init, NearToken::from_near(0));

        ext_oneshot_condvar::ext_on(auth_call)
            .with_static_gas(WAIT_GAS)
            .with_unused_gas_weight(0)
            .cv_wait()
    }

    pub(crate) fn parse_authorization_result() -> bool {
        match env::promise_result(0) {
            PromiseResult::Successful(value) => {
                serde_json::from_slice::<bool>(&value).unwrap_or(false)
            }
            PromiseResult::Failed => false,
        }
    }
}

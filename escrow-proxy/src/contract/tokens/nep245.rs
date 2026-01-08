use defuse_nep245::{ext_mt_core, receiver::MultiTokenReceiver};
use defuse_oneshot_condvar::{WAIT_GAS, ext_oneshot_condvar};
use near_sdk::{
    AccountId, Gas, NearToken, PromiseOrValue, PromiseResult, env, json_types::U128, near, require,
    serde_json,
};

use crate::MT_ON_TRANSFER_GAS;
use crate::contract::{Contract, ContractExt};

#[near]
impl MultiTokenReceiver for Contract {
    #[allow(clippy::used_underscore_binding)]
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        require!(
            env::prepaid_gas() >= MT_ON_TRANSFER_GAS,
            "Insufficient gas prepaid"
        );

        let _ = previous_owner_ids;
        let token_contract = env::predecessor_account_id();
        let (transfer_message, auth_call) =
            self.create_auth_call(&sender_id, &token_ids, &amounts, &msg);

        PromiseOrValue::Promise(
            ext_oneshot_condvar::ext_on(auth_call)
                .with_static_gas(WAIT_GAS)
                .with_unused_gas_weight(0)
                .cv_wait()
                .then(
                    Self::ext(env::current_account_id())
                        .with_unused_gas_weight(1)
                        .check_authorization_and_forward_mt(
                            token_contract,
                            transfer_message.receiver_id,
                            token_ids,
                            amounts,
                            transfer_message.msg,
                        ),
                ),
        )
    }
}

#[near]
impl Contract {
    #[private]
    pub fn check_authorization_and_forward_mt(
        &self,
        token_contract: AccountId,
        escrow_address: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        if !Self::parse_authorization_result() {
            near_sdk::env::panic_str("Authorization failed or timed out, refunding");
        }

        // Forward tokens to escrow
        PromiseOrValue::Promise(
            ext_mt_core::ext(token_contract)
                .with_attached_deposit(NearToken::from_yoctonear(1))
                .with_static_gas(Gas::from_tgas(50))
                .mt_batch_transfer_call(
                    escrow_address,
                    token_ids,
                    amounts.clone(),
                    None,                              // approval
                    Some("proxy forward".to_string()), // memo
                    msg,
                )
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(Gas::from_tgas(10))
                        .resolve_mt_transfer(amounts),
                ),
        )
    }

    #[private]
    pub fn resolve_mt_transfer(&self, original_amounts: Vec<U128>) -> Vec<U128> {
        match env::promise_result(0) {
            PromiseResult::Successful(transferred) => {
                let transferred: Vec<U128> =
                    serde_json::from_slice(&transferred).unwrap_or_else(|_| {
                        near_sdk::log!("Failed to parse escrow response, refunding all");
                        vec![U128(0); original_amounts.len()]
                    });

                original_amounts
                    .iter()
                    .zip(transferred.iter())
                    .map(|(original, transferred)| U128(original.0.saturating_sub(transferred.0)))
                    .collect()
            }
            PromiseResult::Failed => {
                near_sdk::log!("Escrow transfer failed, refunding all");
                original_amounts
            }
        }
    }
}

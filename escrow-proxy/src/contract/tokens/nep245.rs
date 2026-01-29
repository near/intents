use defuse_near_utils::{UnwrapOrPanicError, promise_result_bool, promise_result_vec_U128};
use defuse_nep245::{ext_mt_core, receiver::MultiTokenReceiver};
use near_sdk::{AccountId, Gas, NearToken, PromiseOrValue, env, json_types::U128, near};

const MT_RESOLVE_TRANSFER_GAS: Gas = Gas::from_tgas(10);

use crate::contract::{Contract, ContractExt};
use crate::message::TransferMessage;

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
        let _ = previous_owner_ids;
        let token_contract = env::predecessor_account_id();
        let transfer_message: TransferMessage = msg.parse().unwrap_or_panic_display();

        PromiseOrValue::Promise(
            self.wait_for_authorization(
                &sender_id,
                &token_ids,
                &amounts,
                transfer_message.salt,
                &msg,
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_unused_gas_weight(1)
                    //NOTE: forward all gas, make sure that there is enough gas to resolve transfer
                    .with_static_gas(MT_RESOLVE_TRANSFER_GAS)
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
        if !promise_result_bool(0).unwrap_or(false) {
            near_sdk::env::panic_str("Authorization failed or timed out, refunding");
        }

        PromiseOrValue::Promise(
            ext_mt_core::ext(token_contract)
                .with_attached_deposit(NearToken::from_yoctonear(1))
                .with_unused_gas_weight(1)
                .mt_batch_transfer_call(
                    escrow_address,
                    token_ids,
                    amounts.clone(),
                    None,
                    Some(super::PROXY_MEMO.to_string()),
                    msg,
                )
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(MT_RESOLVE_TRANSFER_GAS)
                        .with_unused_gas_weight(0)
                        .resolve_mt_transfer(amounts),
                ),
        )
    }

    #[private]
    pub fn resolve_mt_transfer(&self, original_amounts: Vec<U128>) -> Vec<U128> {
        let used = promise_result_vec_U128(0, original_amounts.len()).unwrap_or_default();

        original_amounts
            .iter()
            .zip(used.iter())
            .map(|(original, transferred)| U128(original.0.saturating_sub(transferred.0)))
            .collect()
    }
}

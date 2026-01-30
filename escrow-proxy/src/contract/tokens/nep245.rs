use defuse_near_utils::{
    UnwrapOrPanicError, bounded_promise_result, bounded_promise_result_with_args,
};
use defuse_nep245::{ext_mt_core, receiver::MultiTokenReceiver};
use near_sdk::{AccountId, Gas, NearToken, PromiseOrValue, env, json_types::U128, near};

const MT_RESOLVE_TRANSFER_GAS: Gas = Gas::from_tgas(10);
const MT_TRANSFER_CALL_GAS: Gas = Gas::from_tgas(50);
const MT_CHECK_AND_FORWARD_GAS: Gas =
    Gas::from_tgas(MT_RESOLVE_TRANSFER_GAS.as_tgas() + MT_TRANSFER_CALL_GAS.as_tgas());

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
        let token = env::predecessor_account_id();
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
                    .with_static_gas(MT_CHECK_AND_FORWARD_GAS)
                    .check_authorization_and_forward_mt(
                        token,
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
        token: AccountId,
        receiver_id: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        if !bounded_promise_result::<bool>(0).unwrap_or(false) {
            near_sdk::env::panic_str("Authorization failed or timed out, refunding");
        }

        PromiseOrValue::Promise(
            ext_mt_core::ext(token)
                .with_attached_deposit(NearToken::from_yoctonear(1))
                .with_unused_gas_weight(1)
                .with_static_gas(MT_TRANSFER_CALL_GAS)
                .mt_batch_transfer_call(
                    receiver_id,
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
        let used = bounded_promise_result_with_args::<Vec<U128>>(0, original_amounts.len())
            .filter(|v| v.len() == original_amounts.len())
            .unwrap_or(original_amounts.clone());

        original_amounts
            .iter()
            .zip(used.iter())
            .map(|(original, transferred)| U128(original.0.saturating_sub(transferred.0)))
            .collect()
    }
}

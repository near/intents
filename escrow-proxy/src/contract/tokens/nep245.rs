use defuse_near_utils::{
    UnwrapOrPanicError, promise_result_checked_json, promise_result_checked_json_with_args,
};
use defuse_nep245::{ext_mt_core, receiver::MultiTokenReceiver};
use near_sdk::{AccountId, Gas, NearToken, PromiseOrValue, env, json_types::U128, near};

const MT_RESOLVE_FORWARD_GAS: Gas = Gas::from_tgas(5);
const MT_TRANSFER_CALL_MIN_GAS: Gas = Gas::from_tgas(30);
const MT_CHECK_AND_FORWARD_MIN_GAS: Gas = Gas::from_tgas(5)
    .saturating_add(MT_TRANSFER_CALL_MIN_GAS)
    .saturating_add(MT_RESOLVE_FORWARD_GAS);

use crate::contract::{Contract, ContractExt};
use crate::message::ForwardRequest;

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
        let forward_request: ForwardRequest = msg.parse().unwrap_or_panic_display();

        PromiseOrValue::Promise(
            self.wait_for_authorization(&sender_id, &token_ids, &amounts, &msg)
                .then(
                    Self::ext(env::current_account_id())
                        //NOTE: forward all gas, make sure that there is enough gas to resolve transfer
                        .with_static_gas(MT_CHECK_AND_FORWARD_MIN_GAS)
                        .with_unused_gas_weight(1)
                        .mt_forward_checked(
                            token,
                            forward_request.receiver_id,
                            token_ids,
                            amounts,
                            forward_request.msg,
                        ),
                ),
        )
    }
}

#[near]
impl Contract {
    #[private]
    pub fn mt_forward_checked(
        &self,
        token: AccountId,
        receiver_id: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        if !promise_result_checked_json::<bool>(0).unwrap_or(false) {
            near_sdk::env::panic_str("Authorization failed or timed out, refunding");
        }

        ext_mt_core::ext(token)
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(MT_TRANSFER_CALL_MIN_GAS)
            .with_unused_gas_weight(1)
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
                    .with_static_gas(MT_RESOLVE_FORWARD_GAS)
                    .with_unused_gas_weight(0)
                    .mt_resolve_forward(amounts),
            )
            .into()
    }

    #[private]
    pub fn mt_resolve_forward(&self, amounts: Vec<U128>) -> Vec<U128> {
        let used = promise_result_checked_json_with_args::<Vec<U128>>(0, amounts.len())
            // Do not refund on failed `mt_transfer_call`. A known out-of-gas attack
            // makes it impossible to distinguish whether the failure occurred in
            // `mt_transfer_call` itself or in `mt_resolve_transfer` — the resolve
            // function for the `mt_on_transfer` callback. Since `mt_resolve_transfer`
            // is responsible for managing account balances and vulnerability allows for
            // opting out from that logic we choose to lock funds on the
            // proxy account instead of refunding them.
            .filter(|v| v.len() == amounts.len())
            .unwrap_or(amounts.clone());

        amounts
            .iter()
            .zip(used.iter())
            .map(|(original, transferred)| U128(original.0.saturating_sub(transferred.0)))
            .collect()
    }
}

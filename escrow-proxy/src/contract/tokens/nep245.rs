use defuse_near_utils::{
    UnwrapOrPanicError, promise_result_checked_json, promise_result_checked_json_with_len,
};
use defuse_nep245::{ext_mt_core, receiver::MultiTokenReceiver};
use defuse_token_id::TokenId;
use defuse_token_id::nep245::Nep245TokenId;
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

        let wrapped_token_ids: Vec<String> = token_ids
            .iter()
            .cloned()
            .map(|id| TokenId::from(Nep245TokenId::new(token.clone(), id)).to_string())
            .collect();

        self.wait_for_approval(
            &sender_id,
            &wrapped_token_ids,
            &amounts,
            &forward_request.receiver_id,
            &forward_request.msg,
        )
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
        )
        .into()
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
        let authorized = promise_result_checked_json::<bool>(0)
            .ok()
            .and_then(|inner| inner.ok())
            .unwrap_or_default();
        if !authorized {
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
        let mut used = match promise_result_checked_json_with_len::<Vec<U128>>(0, amounts.len()) {
            Ok(Ok(used)) if used.len() == amounts.len() => used,
            Ok(_deserialize_err) => vec![U128(0); amounts.len()],
            // do not refund on failed `mt_batch_transfer_call` due to
            // NEP-141 vulnerability: `mt_resolve_transfer` fails to
            // read result of `mt_on_transfer` due to insufficient gas
            Err(_) => amounts.clone(),
        };

        amounts
            .iter()
            .zip(used.iter())
            .map(|(original, used)| U128(original.0.saturating_sub(used.0)))
            .collect()
    }
}

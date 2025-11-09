use defuse_core::token_id::{TokenId as CoreTokenId, nep245::Nep245TokenId};
use defuse_near_utils::{
    CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic, UnwrapOrPanicError,
};
use defuse_nep245::receiver::{MultiTokenReceiver, ext_mt_receiver};
use near_plugins::{Pausable, pause};
use near_sdk::{AccountId, Gas, Promise, PromiseOrValue, PromiseResult, env, json_types::U128, near, require, serde_json};

use crate::{
    contract::{Contract, ContractExt},
    intents::{Intents, ext_intents},
    tokens::DepositMessage,
};

#[near]
impl MultiTokenReceiver for Contract {
    /// Deposit multi-tokens.
    ///
    /// `msg` contains [`AccountId`] of the internal recipient.
    /// Empty `msg` means deposit to `sender_id`
    #[pause]
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        let token = &*PREDECESSOR_ACCOUNT_ID;
        require!(
            token_ids.len() == amounts.len() && !amounts.is_empty(),
            "invalid args"
        );
        require!(
            token != &*CURRENT_ACCOUNT_ID,
            "self-wrapping is not allowed"
        );

        let DepositMessage {
            receiver_id,
            execute_intents,
            refund_if_fails,
            message,
        } = if msg.is_empty() {
            DepositMessage::new(sender_id.clone())
        } else {
            msg.parse().unwrap_or_panic_display()
        };

        let n = amounts.len();
        let token_ids_clone = token_ids.clone();
        let amounts_clone = amounts.clone();

        self.deposit(
            receiver_id.clone(),
            token_ids.iter()
                .cloned()
                .map(|token_id| Nep245TokenId::new(token.clone(), token_id))
                .map(UnwrapOrPanicError::unwrap_or_panic_display)
                .map(Into::into)
                .zip(amounts.iter().map(|a| a.0)),
            Some("deposit"),
        )
        .unwrap_or_panic();

        let has_message = !message.is_empty();
        let token_account = PREDECESSOR_ACCOUNT_ID.clone();
        let resolver_receiver_id = receiver_id.clone();

        let intents_promise: Option<Promise> = if execute_intents.is_empty() {
            None
        } else if refund_if_fails {
            self.execute_intents(execute_intents);
            None
        } else {
            Some(ext_intents::ext(CURRENT_ACCOUNT_ID.clone()).execute_intents(execute_intents))
        };

        if !has_message {
            return PromiseOrValue::Value(vec![U128(0); n]);
        }

        let notification = ext_mt_receiver::ext(receiver_id.clone())
            .mt_on_transfer(
                sender_id.clone(),
                previous_owner_ids,
                token_ids_clone.clone(),
                amounts_clone.clone(),
                message,
            );

        let resolution = Self::ext(CURRENT_ACCOUNT_ID.clone())
            .with_static_gas(Self::MT_RESOLVE_DEPOSIT_GAS)
            .with_unused_gas_weight(0)
            .mt_resolve_deposit(
                sender_id,
                resolver_receiver_id,
                token_account,
                token_ids_clone,
                amounts_clone,
            );

        match intents_promise {
            Some(promise) => promise.then(notification).then(resolution).into(),
            None => notification.then(resolution).into(),
        }
    }
}

#[near]
impl Contract {
    const MT_RESOLVE_DEPOSIT_GAS: Gas = Gas::from_tgas(50);

    #[private]
    pub fn mt_resolve_deposit(
        &mut self,
        _sender_id: AccountId,
        receiver_id: AccountId,
        token: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
    ) -> Vec<U128> {
        require!(
            env::predecessor_account_id() == *CURRENT_ACCOUNT_ID,
            "only self"
        );

        let token_count = token_ids.len();
        let zero_refunds = vec![U128(0); token_count];
        let requested_refunds = match env::promise_result(0) {
            PromiseResult::Successful(value) => serde_json::from_slice::<Vec<U128>>(&value)
                .ok()
                .filter(|refunds| refunds.len() == token_count)
                // If receiver returns unparseable/wrong-length data, refund everything to protect sender.
                // This provides safety against malicious receivers in multi-token operations.
                .unwrap_or_else(|| amounts.iter().copied().collect()),
            // Do not refund on failure; rely solely on mt_on_transfer return values.
            // This aligns with NEP-141/171 behavior: if the receiver panics, no refund occurs.
            PromiseResult::Failed => zero_refunds,
        };

        let core_token_ids: Vec<CoreTokenId> = token_ids
            .iter()
            .map(|token_id| {
                CoreTokenId::Nep245(
                    Nep245TokenId::new(token.clone(), token_id.clone()).unwrap_or_panic_display(),
                )
            })
            .collect();

        let deposited_amounts: Vec<u128> = amounts.iter().map(|a| a.0).collect();
        let requested_refunds_u128: Vec<u128> = requested_refunds.iter().map(|r| r.0).collect();

        let actual_refunds = self.resolve_deposit_internal(
            &receiver_id,
            core_token_ids,
            deposited_amounts,
            requested_refunds_u128,
        );

        // Return actual refund amounts - the caller (mt_resolve_transfer) will handle:
        // 1. Withdrawing from receiver's balance in the calling contract
        // 2. Depositing back to sender's balance in the calling contract
        actual_refunds.into_iter().map(U128).collect()
    }
}

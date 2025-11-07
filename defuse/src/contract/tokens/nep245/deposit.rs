use defuse_core::{
    intents::tokens::MtWithdraw,
    token_id::{TokenId as CoreTokenId, nep245::Nep245TokenId},
};
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
        sender_id: AccountId,
        receiver_id: AccountId,
        token: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
    ) -> Vec<U128> {
        require!(
            env::predecessor_account_id() == *CURRENT_ACCOUNT_ID,
            "only self"
        );

        // Parse the refund request from mt_on_transfer result
        let requested_refunds = match env::promise_result(0) {
            PromiseResult::Successful(value) => {
                serde_json::from_slice::<Vec<U128>>(&value)
                    .unwrap_or_else(|_| amounts.clone())
            }
            // As per token standard spec, refund whole amounts in case of failure
            PromiseResult::Failed => amounts.clone(),
        };

        // Build refund list
        let mut refund_token_ids = Vec::new();
        let mut refund_amounts = Vec::new();

        for (idx, (token_id, deposited_amount)) in token_ids.iter().zip(amounts.iter()).enumerate() {
            let requested_refund = requested_refunds
                .get(idx)
                .map(|r| r.0)
                .unwrap_or(deposited_amount.0)
                .min(deposited_amount.0);

            if requested_refund == 0 {
                continue;
            }

            // Build core token ID
            let core_token_id = CoreTokenId::Nep245(
                Nep245TokenId::new(token.clone(), token_id.clone()).unwrap_or_panic_display()
            );

            // Check available balance in receiver's account
            let available = {
                let receiver = self.accounts.get(receiver_id.as_ref());
                receiver
                    .map(|account| {
                        account
                            .as_inner_unchecked()
                            .token_balances
                            .amount_for(&core_token_id)
                    })
                    .unwrap_or(0)
            };

            let refund_amount = requested_refund.min(available);
            if refund_amount > 0 {
                refund_token_ids.push(token_id.clone());
                refund_amounts.push(U128(refund_amount));
            }
        }

        // Perform the refund if there are any tokens to refund
        if !refund_token_ids.is_empty() {
            let withdraw = MtWithdraw {
                token: token.clone(),
                receiver_id: sender_id,
                token_ids: refund_token_ids,
                amounts: refund_amounts,
                memo: Some("refund".to_string()),
                msg: None,
                storage_deposit: None,
                min_gas: None,
            };

            match self
                .internal_mt_withdraw(receiver_id, withdraw, true)
                .unwrap_or_panic()
            {
                PromiseOrValue::Promise(_promise) => {
                    // Promise will execute asynchronously
                }
                PromiseOrValue::Value(_) => {}
            }
        }

        // Return zero refunds (refund was already handled via internal_mt_withdraw)
        vec![U128(0); amounts.len()]
    }
}

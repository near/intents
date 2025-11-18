use defuse_core::token_id::{TokenId as CoreTokenId, nep245::Nep245TokenId};
use defuse_near_utils::{
    CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic, UnwrapOrPanicError,
};
use defuse_nep245::receiver::{MultiTokenReceiver, ext_mt_receiver};
use near_plugins::{Pausable, pause};
use near_sdk::{AccountId, PromiseOrValue, json_types::U128, near, require};

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

        let wrapped_tokens: Vec<CoreTokenId> = token_ids
            .iter()
            .map(|token_id| Nep245TokenId::new(token.clone(), token_id.clone()))
            .map(UnwrapOrPanicError::unwrap_or_panic_display)
            .map(Into::into)
            .collect();
        let native_amounts = amounts.iter().map(|elem| elem.0).collect::<Vec<_>>();

        self.deposit(
            receiver_id.clone(),
            wrapped_tokens
                .clone()
                .into_iter()
                .zip(native_amounts.clone()),
            Some("deposit"),
        )
        .unwrap_or_panic();


        if message.as_ref().is_none_or(String::is_empty) {
            return PromiseOrValue::Value(vec![U128(0); token_ids.len()]);
        }

        let notification = ext_mt_receiver::ext(receiver_id.clone()).mt_on_transfer(
            sender_id,
            previous_owner_ids,
            token_ids,
            amounts,
            message.unwrap(),
        );

        let resolution = Self::ext(CURRENT_ACCOUNT_ID.clone())
            .with_static_gas(Self::mt_resolve_deposit_gas(wrapped_tokens.len()))
            .with_unused_gas_weight(0)
            .mt_resolve_deposit(&receiver_id, wrapped_tokens, native_amounts);


       if execute_intents.is_empty() {
            notification.then(resolution).into()
        }else{
            if refund_if_fails {
                self.execute_intents(execute_intents);
                notification.then(resolution).into()
            } else {
                ext_intents::ext(CURRENT_ACCOUNT_ID.clone()).execute_intents(execute_intents)
                .then(notification).then(resolution).into()
            }
        }
  
    }
}

#[near]
impl Contract {
    #[private]
    pub fn mt_resolve_deposit(
        &mut self,
        receiver_id: &AccountId,
        token_ids: Vec<CoreTokenId>,
        deposited_amounts: Vec<u128>,
    ) -> PromiseOrValue<Vec<U128>> {
        PromiseOrValue::Value(self.resolve_deposit_internal(
            receiver_id,
            token_ids,
            deposited_amounts,
        ))
    }
}

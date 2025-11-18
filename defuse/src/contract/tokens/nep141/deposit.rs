use defuse_core::token_id::{TokenId as CoreTokenId, nep141::Nep141TokenId};
use defuse_near_utils::{
    CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic, UnwrapOrPanicError,
};
use defuse_nep245::receiver::ext_mt_receiver;
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_plugins::{Pausable, pause};
use near_sdk::{AccountId, Promise, PromiseOrValue, json_types::U128, near, require};

use crate::{
    contract::{Contract, ContractExt},
    intents::{Intents, ext_intents},
    tokens::DepositMessage,
};

#[near]
impl FungibleTokenReceiver for Contract {
    /// Deposit fungible tokens.
    ///
    /// `msg` contains [`AccountId`] of the internal recipient.
    /// Empty `msg` means deposit to `sender_id`
    #[pause]
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let amount_value = amount.0;
        require!(amount_value > 0, "zero amount");

        let token_id = CoreTokenId::Nep141(Nep141TokenId::new(PREDECESSOR_ACCOUNT_ID.clone()));

        let deposit_message = if msg.is_empty() {
            DepositMessage::new(sender_id)
        } else {
            msg.parse().unwrap_or_panic_display()
        };

        self.deposit(
            deposit_message.receiver_id.clone(),
            [(
                Nep141TokenId::new(PREDECESSOR_ACCOUNT_ID.clone()).into(),
                amount_value,
            )],
            Some("deposit"),
        )
        .unwrap_or_panic();

        match &deposit_message {
            DepositMessage {
                execute_intents,
                message: None,
                ..
            } if execute_intents.is_empty() => PromiseOrValue::Value(U128(0)),
            DepositMessage {
                refund_if_fails: true,
                message: None,
                ..
            } => {
                self.execute_intents(deposit_message.execute_intents);
                PromiseOrValue::Value(U128(0))
            }
            DepositMessage {
                refund_if_fails: false,
                message: None,
                ..
            } => {
                let _ = ext_intents::ext(CURRENT_ACCOUNT_ID.clone())
                    .execute_intents(deposit_message.execute_intents);
                PromiseOrValue::Value(U128(0))
            }
            DepositMessage {
                message: Some(_), ..
            } => self.handle_deposit_with_notification(deposit_message, token_id, amount_value),
        }
    }
}

impl Contract {
    fn handle_deposit_with_notification(
        &mut self,
        deposit_message: DepositMessage,
        token_id: CoreTokenId,
        amount_value: u128,
    ) -> PromiseOrValue<U128> {
        let notification = ext_mt_receiver::ext(deposit_message.receiver_id.clone())
            .mt_on_transfer(
                deposit_message.receiver_id.clone(),
                vec![deposit_message.receiver_id.clone()],
                vec![token_id.to_string()],
                vec![U128(amount_value)],
                deposit_message.message.unwrap(),
            );

        let resolution = Self::ext(CURRENT_ACCOUNT_ID.clone())
            .with_static_gas(Self::mt_resolve_deposit_gas(1))
            .with_unused_gas_weight(0)
            .ft_resolve_deposit(
                &deposit_message.receiver_id,
                vec![token_id],
                vec![amount_value],
            );

        if deposit_message.execute_intents.is_empty() {
            notification.then(resolution).into()
        } else {
            if deposit_message.refund_if_fails {
                self.execute_intents(deposit_message.execute_intents);
                notification.then(resolution).into()
            } else {
                ext_intents::ext(CURRENT_ACCOUNT_ID.clone())
                    .execute_intents(deposit_message.execute_intents)
                    .then(notification)
                    .then(resolution)
                    .into()
            }
        }
    }
}

#[near]
impl Contract {
    #[private]
    pub fn ft_resolve_deposit(
        &mut self,
        receiver_id: &AccountId,
        token_ids: Vec<CoreTokenId>,
        deposited_amounts: Vec<u128>,
    ) -> PromiseOrValue<U128> {
        self.resolve_deposit_internal(receiver_id, token_ids, deposited_amounts)
            .first()
            .map(|elem| PromiseOrValue::Value(*elem))
            .unwrap()
    }
}

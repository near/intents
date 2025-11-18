use defuse_core::token_id::{TokenId as CoreTokenId, nep171::Nep171TokenId};
use defuse_near_utils::{
    CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic, UnwrapOrPanicError,
};
use defuse_nep245::{TokenId, receiver::ext_mt_receiver};
use near_contract_standards::non_fungible_token::core::NonFungibleTokenReceiver;
use near_plugins::{Pausable, pause};
use near_sdk::{AccountId, PromiseOrValue, near};

use crate::{
    contract::{Contract, ContractExt},
    intents::{Intents, ext_intents},
    tokens::DepositMessage,
};

#[near]
impl NonFungibleTokenReceiver for Contract {
    /// Deposit non-fungible token.
    ///
    /// `msg` contains [`AccountId`] of the internal recipient.
    /// Empty `msg` means deposit to `sender_id`
    #[pause]
    fn nft_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_id: AccountId,
        token_id: TokenId,
        msg: String,
    ) -> PromiseOrValue<bool> {
        #[allow(clippy::no_effect_underscore_binding)]
        let _previous_owner_id = previous_owner_id;

        let deposit_message = if msg.is_empty() {
            DepositMessage::new(sender_id.clone())
        } else {
            msg.parse().unwrap_or_panic_display()
        };

        let core_token_id: CoreTokenId =
            Nep171TokenId::new(PREDECESSOR_ACCOUNT_ID.clone(), token_id)
                .unwrap_or_panic_display()
                .into();

        self.deposit(
            deposit_message.receiver_id.clone(),
            [(core_token_id.clone(), 1)],
            Some("deposit"),
        )
        .unwrap_or_panic();

        match deposit_message {
            DepositMessage {
                execute_intents,
                message: None,
                ..
            } if execute_intents.is_empty() => PromiseOrValue::Value(false),
            DepositMessage {
                refund_if_fails: true,
                message: None,
                execute_intents,
                ..
            } => {
                self.execute_intents(execute_intents);
                PromiseOrValue::Value(false)
            }
            DepositMessage {
                refund_if_fails: false,
                message: None,
                execute_intents,
                ..
            } => {
                let _ =
                    ext_intents::ext(CURRENT_ACCOUNT_ID.clone()).execute_intents(execute_intents);
                PromiseOrValue::Value(false)
            }
            DepositMessage {
                message: Some(_), ..
            } => {
                self.handle_nft_deposit_with_notification(deposit_message, sender_id, core_token_id)
            }
        }
    }
}

impl Contract {
    fn handle_nft_deposit_with_notification(
        &mut self,
        deposit_message: DepositMessage,
        sender_id: AccountId,
        core_token_id: CoreTokenId,
    ) -> PromiseOrValue<bool> {
        let notification = ext_mt_receiver::ext(deposit_message.receiver_id.clone())
            .mt_on_transfer(
                sender_id.clone(),
                vec![sender_id],
                vec![core_token_id.to_string()],
                vec![near_sdk::json_types::U128(1)],
                deposit_message.message.unwrap(),
            );

        let resolution = Self::ext(CURRENT_ACCOUNT_ID.clone())
            .with_static_gas(Self::mt_resolve_deposit_gas(1))
            .with_unused_gas_weight(0)
            .nft_resolve_deposit(&deposit_message.receiver_id, vec![core_token_id], vec![1]);

        if deposit_message.execute_intents.is_empty() {
            notification.then(resolution).into()
        } else if deposit_message.refund_if_fails {
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

#[near]
impl Contract {
    #[private]
    pub fn nft_resolve_deposit(
        &mut self,
        receiver_id: &AccountId,
        token_ids: Vec<CoreTokenId>,
        deposited_amounts: Vec<u128>,
    ) -> PromiseOrValue<bool> {
        self.resolve_deposit_internal(receiver_id, token_ids, deposited_amounts)
            .first()
            .copied()
            .map(|elem| PromiseOrValue::Value(elem == 1.into()))
            .unwrap()
    }
}

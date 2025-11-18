use defuse_core::token_id::{TokenId as CoreTokenId, nep141::Nep141TokenId};
use defuse_near_utils::{
    CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic, UnwrapOrPanicError,
};
use defuse_nep245::receiver::ext_mt_receiver;
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_plugins::{Pausable, pause};
use near_sdk::{AccountId, PromiseOrValue, json_types::U128, near, require};

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

        let DepositMessage {
            receiver_id,
            execute_intents,
            refund_if_fails,
            message,
        } = if msg.is_empty() {
            DepositMessage::new(sender_id)
        } else {
            msg.parse().unwrap_or_panic_display()
        };

        self.deposit(
            receiver_id.clone(),
            [(
                Nep141TokenId::new(PREDECESSOR_ACCOUNT_ID.clone()).into(),
                amount_value,
            )],
            Some("deposit"),
        )
        .unwrap_or_panic();

        if message.as_ref().is_none_or(String::is_empty) {
            return PromiseOrValue::Value(U128(0));
        }

        let notification = ext_mt_receiver::ext(receiver_id.clone()).mt_on_transfer(
            receiver_id.clone(),
            vec![receiver_id.clone()],
            vec![token_id.to_string()],
            vec![U128(amount_value)],
            message.unwrap(),
        );

        let resolution = Self::ext(CURRENT_ACCOUNT_ID.clone())
            .with_static_gas(Self::mt_resolve_deposit_gas(1))
            .with_unused_gas_weight(0)
            .ft_resolve_deposit(&receiver_id, vec![token_id], vec![amount_value]);

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

use defuse_core::token_id::{TokenId, nep141::Nep141TokenId};
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
    tokens::{DepositAction, DepositMessage},
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

        let token_id = TokenId::Nep141(Nep141TokenId::new(PREDECESSOR_ACCOUNT_ID.clone()));

        let DepositMessage {
            receiver_id,
            action,
        } = if msg.is_empty() {
            DepositMessage::new(sender_id.clone())
        } else {
            msg.parse().unwrap_or_panic_display()
        };

        self.deposit(
            receiver_id.clone(),
            [(token_id.clone(), amount_value)],
            Some("deposit"),
        )
        .unwrap_or_panic();

        let Some(action) = action else {
            return PromiseOrValue::Value(0.into());
        };

        match action {
            DepositAction::Notify(notify) => ext_mt_receiver::ext(receiver_id.clone())
                .with_static_gas(notify.min_gas.unwrap_or_default())
                .mt_on_transfer(
                    sender_id.clone(),
                    vec![sender_id],
                    vec![token_id.to_string()],
                    vec![amount],
                    notify.msg,
                )
                .then(
                    Self::ext(CURRENT_ACCOUNT_ID.clone())
                        .with_static_gas(Self::mt_resolve_deposit_gas(1))
                        .with_unused_gas_weight(0)
                        .ft_resolve_deposit(&receiver_id, token_id, amount),
                )
                .into(),
            DepositAction::Execute(execute) => {
                if execute.execute_intents.is_empty() {
                    return PromiseOrValue::Value(0.into());
                }

                if execute.refund_if_fails {
                    self.execute_intents(execute.execute_intents);
                } else {
                    // detach promise
                    let _ = ext_intents::ext(CURRENT_ACCOUNT_ID.clone())
                        .execute_intents(execute.execute_intents);
                }
                PromiseOrValue::Value(0.into())
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
        token_ids: TokenId,
        deposited_amounts: U128,
    ) -> PromiseOrValue<U128> {
        let [result] = self
            .resolve_deposit_internal(receiver_id, &[token_ids], &[deposited_amounts.0])
            .try_into()
            .unwrap_or_else(|_| {
                unreachable!("ft_resolve_deposit expects return value of length == 1")
            });
        PromiseOrValue::Value(result)
    }
}

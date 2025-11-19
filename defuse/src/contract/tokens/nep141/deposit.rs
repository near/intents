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
    tokens::{DepositMessage, DepositMessageV2, DepositMessageActionV2},
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

        let DepositMessageV2 {
            receiver_id,
            action,
        } = if msg.is_empty() {
            DepositMessage::new(sender_id)
        } else {
            msg.parse().unwrap_or_panic_display()
        }
        .into_v2();

        self.deposit(
            receiver_id.clone(),
            [(token_id.clone(), amount_value)],
            Some("deposit"),
        )
        .unwrap_or_panic();

        match action {
            Some(DepositMessageActionV2::Notify(notify)) => {
                let mut on_transfer = ext_mt_receiver::ext(receiver_id.clone());

                if let Some(gas) = notify.min_gas {
                    on_transfer = on_transfer.with_static_gas(gas);
                }

                let on_transfer = on_transfer.mt_on_transfer(
                    receiver_id.clone(),
                    vec![receiver_id.clone()],
                    vec![token_id.to_string()],
                    vec![U128(amount_value)],
                    notify.msg,
                );

                let resolution = Self::ext(CURRENT_ACCOUNT_ID.clone())
                    .with_static_gas(Self::mt_resolve_deposit_gas(1))
                    .with_unused_gas_weight(0)
                    .ft_resolve_deposit(&receiver_id, token_id, amount_value);

                on_transfer.then(resolution).into()
            }
            Some(DepositMessageActionV2::Execute(execute)) => {
                if execute.refund_if_fails {
                    self.execute_intents(execute.execute_intents);
                } else {
                    // detach promise
                    let _ = ext_intents::ext(CURRENT_ACCOUNT_ID.clone())
                        .execute_intents(execute.execute_intents);
                }
                PromiseOrValue::Value(0.into())
            }
            None => PromiseOrValue::Value(0.into()),
        }
    }
}

#[near]
impl Contract {
    #[private]
    pub fn ft_resolve_deposit(
        &mut self,
        receiver_id: &AccountId,
        token_ids: CoreTokenId,
        deposited_amounts: u128,
    ) -> PromiseOrValue<U128> {
        let [result] = self
            .resolve_deposit_internal(receiver_id, vec![token_ids], vec![deposited_amounts])
            .try_into()
            .unwrap_or_else(|_| {
                unreachable!("ft_resolve_deposit expects return value of length == 1")
            });
        PromiseOrValue::Value(result)
    }
}

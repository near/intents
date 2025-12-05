use defuse_core::token_id::{TokenId, nep141::Nep141TokenId};
use defuse_near_utils::{
    CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic, UnwrapOrPanicError,
};
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
        require!(amount.0 > 0, "zero amount");

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
            [(token_id.clone(), amount.0)],
            Some("deposit"),
        )
        .unwrap_or_panic();

        let Some(action) = action else {
            return PromiseOrValue::Value(0.into());
        };

        match action {
            DepositAction::Notify(notify) => Self::notify_on_transfer(
                sender_id.clone(),
                vec![sender_id],
                receiver_id.clone(),
                vec![token_id.to_string()],
                vec![amount],
                notify,
            )
            .then(
                Self::ext(CURRENT_ACCOUNT_ID.clone())
                    .with_static_gas(Self::mt_resolve_deposit_gas(1))
                    .with_unused_gas_weight(0)
                    .ft_resolve_deposit(receiver_id, PREDECESSOR_ACCOUNT_ID.clone(), amount),
            )
            .into(),
            DepositAction::Execute(execute) => {
                if !execute.execute_intents.is_empty() {
                    if execute.refund_if_fails {
                        self.execute_intents(execute.execute_intents);
                    } else {
                        ext_intents::ext(CURRENT_ACCOUNT_ID.clone())
                            .execute_intents(execute.execute_intents)
                            .detach();
                    }
                }
                PromiseOrValue::Value(0.into())
            }
        }
    }
}

#[near]
impl Contract {
    #[private]
    #[allow(clippy::needless_pass_by_value)]
    pub fn ft_resolve_deposit(
        &mut self,
        receiver_id: AccountId,
        contract_id: AccountId,
        #[allow(unused_mut)] mut amount: U128,
    ) -> PromiseOrValue<U128> {
        self.resolve_deposit_internal(
            &receiver_id,
            [(Nep141TokenId::new(contract_id).into(), &mut amount.0)],
        );
        PromiseOrValue::Value(amount)
    }
}

use defuse_core::token_id::{TokenId, nep245::Nep245TokenId};
use defuse_near_utils::{
    CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic, UnwrapOrPanicError,
};
use defuse_nep245::receiver::{MultiTokenReceiver, ext_mt_receiver};
use near_plugins::{Pausable, pause};
use near_sdk::{AccountId, PromiseOrValue, json_types::U128, near, require};

use crate::{
    contract::{Contract, ContractExt},
    intents::{Intents, ext_intents},
    tokens::{DepositAction, DepositMessage},
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

        require!(!amounts.is_empty(), "invalid args");

        require!(
            token_ids.len() == amounts.len(),
            "NEP-245: Contract MUST panic if `token_ids` length does not equals `amounts` length"
        );

        require!(
            previous_owner_ids.len() == token_ids.len(),
            "NEP-245: Contract MUST panic if `previous_owner_ids` length does not equals `token_ids` length"
        );

        require!(
            token != &*CURRENT_ACCOUNT_ID,
            "self-wrapping is not allowed"
        );

        let DepositMessage {
            receiver_id,
            action,
        } = if msg.is_empty() {
            DepositMessage::new(sender_id.clone())
        } else {
            msg.parse().unwrap_or_panic_display()
        };

        let wrapped_tokens: Vec<TokenId> = token_ids
            .iter()
            .map(|token_id| Nep245TokenId::new(token.clone(), token_id.clone()))
            .map(UnwrapOrPanicError::unwrap_or_panic_display)
            .map(Into::into)
            .collect();
        let native_amounts = amounts.iter().map(|elem| elem.0).collect::<Vec<_>>();

        self.deposit(
            receiver_id.clone(),
            wrapped_tokens.clone().into_iter().zip(native_amounts),
            Some("deposit"),
        )
        .unwrap_or_panic();

        let Some(action) = action else {
            return PromiseOrValue::Value(vec![U128(0); token_ids.len()]);
        };

        match action {
            DepositAction::Notify(notify) => ext_mt_receiver::ext(receiver_id.clone())
                .with_static_gas(notify.min_gas.unwrap_or_default())
                .mt_on_transfer(
                    sender_id,
                    previous_owner_ids,
                    token_ids,
                    amounts.clone(),
                    notify.msg,
                )
                .then(
                    Self::ext(CURRENT_ACCOUNT_ID.clone())
                        .with_static_gas(Self::mt_resolve_deposit_gas(wrapped_tokens.len()))
                        .with_unused_gas_weight(0)
                        .mt_resolve_deposit(&receiver_id, wrapped_tokens, amounts),
                )
                .into(),
            DepositAction::Execute(execute) => {
                if execute.execute_intents.is_empty() {
                    return PromiseOrValue::Value(vec![U128(0); token_ids.len()]);
                }

                if execute.refund_if_fails {
                    self.execute_intents(execute.execute_intents);
                } else {
                    let _ = ext_intents::ext(CURRENT_ACCOUNT_ID.clone())
                        .execute_intents(execute.execute_intents);
                }

                PromiseOrValue::Value(vec![U128(0); token_ids.len()])
            }
        }
    }
}

#[near]
impl Contract {
    #[private]
    #[allow(clippy::needless_pass_by_value)]
    pub fn mt_resolve_deposit(
        &mut self,
        receiver_id: &AccountId,
        token_ids: Vec<TokenId>,
        deposited_amounts: Vec<U128>,
    ) -> PromiseOrValue<Vec<U128>> {
        let tokens_count = token_ids.len();

        let amounts_vec: Vec<u128> = deposited_amounts.iter().map(|val| val.0).collect();
        let result = self.resolve_deposit_internal(receiver_id, &token_ids, &amounts_vec);

        if result.len() != tokens_count {
            unreachable!("mt_resolve_deposit expects return value of length == token_ids.len()");
        }

        PromiseOrValue::Value(result)
    }
}

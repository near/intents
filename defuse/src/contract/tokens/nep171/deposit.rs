use defuse_core::token_id::{TokenId, nep171::Nep171TokenId};
use defuse_near_utils::{
    CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic, UnwrapOrPanicError,
};
use defuse_nep245::receiver::ext_mt_receiver;
use near_contract_standards::non_fungible_token::core::NonFungibleTokenReceiver;
use near_plugins::{Pausable, pause};
use near_sdk::{AccountId, PromiseOrValue, json_types::U128, near};

use crate::{
    contract::{Contract, ContractExt},
    intents::{Intents, ext_intents},
    tokens::{DepositAction, DepositMessage},
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
        token_id: defuse_nep245::TokenId,
        msg: String,
    ) -> PromiseOrValue<bool> {
        let DepositMessage {
            receiver_id,
            action,
        } = if msg.is_empty() {
            DepositMessage::new(sender_id.clone())
        } else {
            msg.parse().unwrap_or_panic_display()
        };

        let core_token_id: TokenId = Nep171TokenId::new(PREDECESSOR_ACCOUNT_ID.clone(), token_id)
            .unwrap_or_panic_display()
            .into();

        self.deposit(
            receiver_id.clone(),
            [(core_token_id.clone(), 1)],
            Some("deposit"),
        )
        .unwrap_or_panic();

        let Some(action) = action else {
            return PromiseOrValue::Value(false);
        };

        match action {
            DepositAction::Notify(notify) => ext_mt_receiver::ext(receiver_id.clone())
                .with_static_gas(notify.min_gas.unwrap_or_default())
                .mt_on_transfer(
                    sender_id,
                    vec![previous_owner_id],
                    vec![core_token_id.to_string()],
                    vec![U128(1)],
                    notify.msg,
                )
                .then(
                    Self::ext(CURRENT_ACCOUNT_ID.clone())
                        .with_static_gas(Self::mt_resolve_deposit_gas(1))
                        .with_unused_gas_weight(0)
                        .nft_resolve_deposit(&receiver_id, core_token_id),
                )
                .into(),
            DepositAction::Execute(execute) => {
                if execute.execute_intents.is_empty() {
                    return PromiseOrValue::Value(false);
                }

                if execute.refund_if_fails {
                    self.execute_intents(execute.execute_intents);
                } else {
                    let _ = ext_intents::ext(CURRENT_ACCOUNT_ID.clone())
                        .execute_intents(execute.execute_intents);
                }
                PromiseOrValue::Value(false)
            }
        }
    }
}

#[near]
impl Contract {
    #[private]
    pub fn nft_resolve_deposit(
        &mut self,
        receiver_id: &AccountId,
        token_ids: TokenId,
    ) -> PromiseOrValue<bool> {
        let mut amount = 1u128;
        self
            .resolve_deposit_internal(receiver_id, [(token_ids, &mut amount)]);
        PromiseOrValue::Value(amount == 1)
    }
}

use defuse_core::token_id::{TokenId as CoreTokenId, nep171::Nep171TokenId};
use defuse_near_utils::{
    CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic, UnwrapOrPanicError,
};
use defuse_nep245::{TokenId, receiver::ext_mt_receiver};
use near_contract_standards::non_fungible_token::core::NonFungibleTokenReceiver;
use near_plugins::{Pausable, pause};
use near_sdk::{AccountId, PromiseOrValue, json_types::U128, near};

use crate::{
    contract::{Contract, ContractExt},
    intents::{Intents, ext_intents},
    tokens::{DepositMessage, DepositMessageAction},
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

        let DepositMessage {
            receiver_id,
            action,
        } = if msg.is_empty() {
            DepositMessage::new(sender_id.clone())
        } else {
            msg.parse().unwrap_or_panic_display()
        };

        let core_token_id: CoreTokenId =
            Nep171TokenId::new(PREDECESSOR_ACCOUNT_ID.clone(), token_id)
                .unwrap_or_panic_display()
                .into();

        self.deposit(
            receiver_id.clone(),
            [(core_token_id.clone(), 1)],
            Some("deposit"),
        )
        .unwrap_or_panic();

        match action {
            Some(DepositMessageAction::Notify(notify)) => {
                let mut on_transfer = ext_mt_receiver::ext(receiver_id.clone());
                if let Some(gas) = notify.min_gas {
                    on_transfer = on_transfer.with_static_gas(gas);
                }

                let on_transfer = on_transfer.mt_on_transfer(
                    sender_id.clone(),
                    vec![sender_id],
                    vec![core_token_id.to_string()],
                    vec![U128(1)],
                    notify.msg,
                );

                let resolution = Self::ext(CURRENT_ACCOUNT_ID.clone())
                    .with_static_gas(Self::mt_resolve_deposit_gas(1))
                    .with_unused_gas_weight(0)
                    .nft_resolve_deposit(&receiver_id, core_token_id);

                on_transfer.then(resolution).into()
            }
            Some(DepositMessageAction::Execute(execute)) => {
                if execute.refund_if_fails {
                    self.execute_intents(execute.execute_intents);
                } else {
                    let _ = ext_intents::ext(CURRENT_ACCOUNT_ID.clone())
                        .execute_intents(execute.execute_intents);
                }
                PromiseOrValue::Value(false)
            }
            None => PromiseOrValue::Value(false),
        }
    }
}

#[near]
impl Contract {
    #[private]
    pub fn nft_resolve_deposit(
        &mut self,
        receiver_id: &AccountId,
        token_ids: CoreTokenId,
    ) -> PromiseOrValue<bool> {
        let [result] = self
            .resolve_deposit_internal(receiver_id, vec![token_ids], vec![1])
            .try_into()
            .unwrap_or_else(|_| {
                unreachable!("nft_resolve_deposit expects return value of length == 1")
            });
        PromiseOrValue::Value(result == 1.into())
    }
}

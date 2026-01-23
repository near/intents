use defuse_core::token_id::{MAX_TOKEN_ID_LEN, TokenId, TokenIdTooLarge, nep171::Nep171TokenId};
use defuse_near_utils::{PanicError, UnwrapOrPanic, UnwrapOrPanicError};
use near_contract_standards::non_fungible_token::core::NonFungibleTokenReceiver;
use near_plugins::{Pausable, pause};
use near_sdk::{AccountId, PromiseOrValue, env, json_types::U128, near};

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
        token_id: near_contract_standards::non_fungible_token::TokenId,
        msg: String,
    ) -> PromiseOrValue<bool> {
        if token_id.len() > MAX_TOKEN_ID_LEN {
            TokenIdTooLarge(token_id.len()).panic_display();
        }

        let DepositMessage {
            receiver_id,
            action,
        } = if msg.is_empty() {
            DepositMessage::new(sender_id.clone())
        } else {
            msg.parse().unwrap_or_panic_display()
        };

        let core_token_id: TokenId =
            Nep171TokenId::new(env::predecessor_account_id(), token_id.clone()).into();

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
            DepositAction::Notify(notify) => Self::notify_on_transfer(
                sender_id,
                vec![previous_owner_id],
                receiver_id.clone(),
                vec![core_token_id.to_string()],
                vec![U128(1)],
                notify,
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Self::mt_resolve_deposit_gas(1))
                    .with_unused_gas_weight(0)
                    .nft_resolve_deposit(receiver_id, env::predecessor_account_id(), token_id),
            )
            .into(),
            DepositAction::Execute(execute) => {
                if !execute.execute_intents.is_empty() {
                    if execute.refund_if_fails {
                        self.execute_intents(execute.execute_intents);
                    } else {
                        ext_intents::ext(env::current_account_id())
                            .execute_intents(execute.execute_intents)
                            .detach();
                    }
                }

                PromiseOrValue::Value(false)
            }
        }
    }
}

#[near]
impl Contract {
    #[private]
    #[allow(clippy::needless_pass_by_value)]
    pub fn nft_resolve_deposit(
        &mut self,
        receiver_id: AccountId,
        contract_id: AccountId,
        nft_token_id: near_contract_standards::non_fungible_token::TokenId,
    ) -> PromiseOrValue<bool> {
        let mut amount = 1u128;

        self.resolve_deposit_internal(
            &receiver_id,
            [(
                Nep171TokenId::new(contract_id, nft_token_id).into(),
                &mut amount,
            )],
        );
        PromiseOrValue::Value(amount != 0)
    }
}

use defuse_core::tokens::TokenId;
use defuse_near_utils::{
    CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic, UnwrapOrPanicError,
};
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
        token_id: near_contract_standards::non_fungible_token::TokenId,
        msg: String,
    ) -> PromiseOrValue<bool> {
        #[allow(clippy::no_effect_underscore_binding)]
        let _previous_owner_id = previous_owner_id;
        let msg = if msg.is_empty() {
            DepositMessage::new(sender_id)
        } else {
            msg.parse().unwrap_or_panic_display()
        };

        self.deposit(
            msg.receiver_id,
            [(TokenId::Nep171(PREDECESSOR_ACCOUNT_ID.clone(), token_id), 1)],
            Some("deposit"),
        )
        .unwrap_or_panic();

        if !msg.execute_intents.is_empty() {
            if msg.refund_if_fails {
                self.execute_intents(msg.execute_intents);
            } else {
                // detach promise
                let _ = ext_intents::ext(CURRENT_ACCOUNT_ID.clone())
                    .execute_intents(msg.execute_intents);
            }
        }

        PromiseOrValue::Value(false)
    }
}

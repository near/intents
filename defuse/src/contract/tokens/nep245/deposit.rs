use defuse_core::token_id::nep245::Nep245TokenId;
use defuse_near_utils::{
    CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic, UnwrapOrPanicError,
};
use defuse_nep245::receiver::MultiTokenReceiver;
use near_plugins::{Pausable, pause};
use near_sdk::{AccountId, PromiseOrValue, json_types::U128, near, require};

use crate::{
    contract::{Contract, ContractExt},
    intents::{Intents, ext_intents},
    tokens::DepositMessage,
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
        let _previous_owner_ids = previous_owner_ids;
        let token = &*PREDECESSOR_ACCOUNT_ID;
        require!(
            token_ids.len() == amounts.len() && !amounts.is_empty(),
            "invalid args"
        );
        require!(
            token != &*CURRENT_ACCOUNT_ID,
            "self-wrapping is not allowed"
        );
        let msg = if msg.is_empty() {
            DepositMessage::new(sender_id)
        } else {
            msg.parse().unwrap_or_panic_display()
        };

        let n = amounts.len();

        self.deposit(
            msg.receiver_id,
            token_ids
                .into_iter()
                .map(|token_id| Nep245TokenId::new(token.clone(), token_id))
                .map(UnwrapOrPanicError::unwrap_or_panic_display)
                .map(Into::into)
                .zip(amounts.into_iter().map(|a| a.0)),
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

        PromiseOrValue::Value(vec![U128(0); n])
    }
}

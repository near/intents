mod nep141;
mod nep171;
mod nep245;

use super::{Contract, intents::execute::ExecuteInspector};
use defuse_core::{DefuseError, Result, engine::Engine, tokens::TokenId};
use defuse_nep245::MtBurnEvent;
use near_sdk::{AccountId, AccountIdRef, Gas, json_types::U128};
use std::borrow::Cow;

pub const STORAGE_DEPOSIT_GAS: Gas = Gas::from_tgas(10);

impl Contract {
    pub(crate) fn deposit(
        &mut self,
        owner_id: AccountId,
        tokens: impl IntoIterator<Item = (TokenId, u128)>,
        memo: Option<&str>,
    ) -> Result<()> {
        Engine::new(self, ExecuteInspector::default()).deposit(owner_id, tokens, memo)?;

        Ok(())
    }

    pub(crate) fn withdraw(
        &mut self,
        owner_id: &AccountIdRef,
        token_amounts: impl IntoIterator<Item = (TokenId, u128)>,
        memo: Option<impl Into<String>>,
    ) -> Result<()> {
        let owner = self
            .accounts
            .get_mut(owner_id)
            .ok_or(DefuseError::AccountNotFound)?;

        let mut burn_event = MtBurnEvent {
            owner_id: Cow::Owned(owner_id.to_owned()),
            authorized_id: None,
            token_ids: Vec::new().into(),
            amounts: Vec::new().into(),
            memo: memo.map(Into::into).map(Into::into),
        };

        for (token_id, amount) in token_amounts {
            if amount == 0 {
                return Err(DefuseError::InvalidIntent);
            }

            burn_event.token_ids.to_mut().push(token_id.to_string());
            burn_event.amounts.to_mut().push(U128(amount));

            owner
                .token_balances
                .sub(token_id.clone(), amount)
                .ok_or(DefuseError::BalanceOverflow)?;

            self.state
                .total_supplies
                .sub(token_id, amount)
                .ok_or(DefuseError::BalanceOverflow)?;
        }

        // Schedule to emit `mt_burn` events only in the end of tx
        // to avoid confusion when `mt_burn` occurs before relevant
        // `mt_transfer` arrives. This can happen due to postponed
        // delta-matching during intents execution.
        if !burn_event.amounts.is_empty() {
            self.postponed_burns.mt_burn(burn_event);
        }

        Ok(())
    }
}

mod nep141;
mod nep171;
mod nep245;

use super::Contract;
use defuse_core::{DefuseError, Result, token_id::TokenId};
use defuse_nep245::{MtBurnEvent, MtEvent, MtMintEvent};
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
        let owner = self
            .accounts
            .get_or_create(owner_id.clone())
            // deposits are allowed for locked accounts
            .as_inner_unchecked_mut();

        let mut mint_event = MtMintEvent {
            owner_id: owner_id.into(),
            token_ids: Vec::new().into(),
            amounts: Vec::new().into(),
            memo: memo.map(Into::into),
        };

        for (token_id, amount) in tokens {
            if amount == 0 {
                return Err(DefuseError::InvalidIntent);
            }

            mint_event.token_ids.to_mut().push(token_id.to_string());
            mint_event.amounts.to_mut().push(U128(amount));

            let total_supply = self
                .state
                .total_supplies
                .add(token_id.clone(), amount)
                .ok_or(DefuseError::BalanceOverflow)?;
            match token_id {
                TokenId::Nep171(ref tid) => {
                    if total_supply > 1 {
                        return Err(DefuseError::NftAlreadyDeposited(tid.clone()));
                    }
                }
                TokenId::Nep141(_) | TokenId::Nep245(_) => {}
            }
            owner
                .token_balances
                .add(token_id, amount)
                .ok_or(DefuseError::BalanceOverflow)?;
        }

        if !mint_event.amounts.is_empty() {
            MtEvent::MtMint([mint_event].as_slice().into()).emit();
        }

        Ok(())
    }

    pub(crate) fn withdraw(
        &mut self,
        owner_id: &AccountIdRef,
        token_amounts: impl IntoIterator<Item = (TokenId, u128)>,
        memo: Option<impl Into<String>>,
        force: bool,
    ) -> Result<()> {
        let owner = self
            .accounts
            .get_mut(owner_id)
            .ok_or_else(|| DefuseError::AccountNotFound(owner_id.to_owned()))?
            .get_mut_maybe_forced(force)
            .ok_or_else(|| DefuseError::AccountLocked(owner_id.to_owned()))?;

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

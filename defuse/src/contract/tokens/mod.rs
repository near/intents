mod nep141;
mod nep171;
mod nep245;

use super::Contract;
use defuse_core::{DefuseError, Result, token_id::TokenId};
use defuse_nep245::{MtBurnEvent, MtEvent, MtMintEvent};
use near_sdk::{AccountId, AccountIdRef, Gas, json_types::U128};
use std::borrow::Cow;

pub const STORAGE_DEPOSIT_GAS: Gas = Gas::from_tgas(10);

/// In functions of `MultiToken` that emit logs of everything that happens,
/// how many tokens to batch together in one event.
/// A very high number can lead to hitting the log limit in near (16384 chars at the time of writing).
/// A very low number will consume too much gas and make transactions with many tokens fail early.
pub const LOG_CHUNK_TOKEN_COUNT: usize = 25;

impl Contract {
    pub(crate) fn deposit(
        &mut self,
        owner_id: AccountId,
        tokens: impl IntoIterator<Item = (TokenId, u128)>,
        memo: Option<&str>,
    ) -> Result<()> {
        let owner = self.accounts.get_or_create(owner_id.clone());

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
            // We batch logging because there is a limit on the size of logs
            let token_log_batch = mint_event.token_ids.chunks(LOG_CHUNK_TOKEN_COUNT);
            let amount_log_batch = mint_event.amounts.chunks(LOG_CHUNK_TOKEN_COUNT);

            for (token_ids, amounts) in token_log_batch.zip(amount_log_batch) {
                MtEvent::MtMint(
                    [MtMintEvent {
                        owner_id: Cow::Borrowed(&mint_event.owner_id),
                        token_ids: token_ids.into(),
                        amounts: amounts.into(),
                        memo: memo.map(Into::into),
                    }]
                    .as_slice()
                    .into(),
                )
                .emit();
            }
        }

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

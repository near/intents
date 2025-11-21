mod nep141;
mod nep171;
mod nep245;

use super::Contract;
use defuse_core::{DefuseError, Result, token_id::TokenId};
use defuse_near_utils::Lock;
use defuse_nep245::{MtBurnEvent, MtEvent, MtMintEvent};
use itertools::{Either, Itertools};
use near_sdk::{AccountId, AccountIdRef, Gas, PromiseResult, env, json_types::U128, serde_json};
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
            .storage
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
                .storage
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
            .storage
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

            self.storage
                .state
                .total_supplies
                .sub(token_id, amount)
                .ok_or(DefuseError::BalanceOverflow)?;
        }

        // Schedule to emit `mt_burn` events only in the end of tx
        // to avoid confusion when `mt_burn` occurs before relevant
        // `mt_transfer` arrives. This can happen due to postponed
        // delta-matching during intents execution.
        if !burn_event.amounts.is_empty() {
            self.runtime.postponed_burns.mt_burn(burn_event);
        }

        Ok(())
    }
}

impl Contract {
    #[must_use]
    pub(crate) fn mt_resolve_deposit_gas(token_count: usize) -> Gas {
        const MT_RESOLVE_DEPOSIT_PER_TOKEN_GAS: Gas = Gas::from_tgas(2);
        const MT_RESOLVE_DEPOSIT_BASE_GAS: Gas = Gas::from_tgas(4);

        let token_count: u64 = token_count
            .try_into()
            .unwrap_or_else(|_| env::panic_str(&format!("token_count overflow: {token_count}")));

        MT_RESOLVE_DEPOSIT_BASE_GAS
            .checked_add(
                MT_RESOLVE_DEPOSIT_PER_TOKEN_GAS
                    .checked_mul(token_count)
                    .unwrap_or_else(|| env::panic_str("gas calculation overflow")),
            )
            .unwrap_or_else(|| env::panic_str("gas calculation overflow"))
    }

    pub fn resolve_deposit_internal<'a, I>(&mut self, receiver_id: &AccountId, tokens: I)
    where
        I: IntoIterator<Item = (TokenId, &'a mut u128)>,
        I::IntoIter: ExactSizeIterator,
    {
        let tokens_iter = tokens.into_iter();
        let tokens_count = tokens_iter.len();
        let requested_refunds = match env::promise_result(0) {
            PromiseResult::Successful(value) => serde_json::from_slice::<Vec<U128>>(&value)
                .ok()
                .filter(|refunds| refunds.len() == tokens_count),
            PromiseResult::Failed => None,
        };

        let mut burn_event = MtBurnEvent {
            owner_id: Cow::Owned(receiver_id.clone()),
            authorized_id: None,
            token_ids: Vec::with_capacity(tokens_count).into(),
            amounts: Vec::with_capacity(tokens_count).into(),
            memo: Some("refund".into()),
        };

        let Some(receiver) = self
            .storage
            .accounts
            .get_mut(receiver_id.as_ref())
            .map(Lock::as_inner_unchecked_mut)
        else {
            tokens_iter.for_each(|(_, amount)| *amount = 0);
            return;
        };

        for ((token_id, deposited), requested_refund) in
            tokens_iter.zip_eq(requested_refunds.map_or_else(
                || Either::Right(std::iter::repeat_n(None, tokens_count)),
                |v| Either::Left(v.into_iter().map(|elem| Some(elem.0))),
            ))
        {
            //NOTE: refunds are capped by deposited amounts
            let requested_refund = requested_refund.unwrap_or(*deposited);
            let balance_left = receiver.token_balances.amount_for(&token_id);
            let refund_amount = balance_left.min(requested_refund);
            *deposited = refund_amount;
            if refund_amount == 0 {
                continue;
            }

            burn_event.token_ids.to_mut().push(token_id.to_string());
            burn_event.amounts.to_mut().push(U128(refund_amount));

            receiver
                .token_balances
                .sub(token_id.clone(), refund_amount)
                .unwrap_or_else(|| env::panic_str("balance underflow"));

            self.storage
                .state
                .total_supplies
                .sub(token_id, refund_amount)
                .unwrap_or_else(|| env::panic_str("total supply underflow"));
        }

        if !burn_event.amounts.is_empty() {
            MtEvent::MtBurn([burn_event].as_slice().into()).emit();
        }
    }
}

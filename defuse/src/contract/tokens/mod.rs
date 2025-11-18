mod nep141;
mod nep171;
mod nep245;

use super::Contract;
use defuse_core::{DefuseError, Result, token_id::TokenId};
use defuse_near_utils::CURRENT_ACCOUNT_ID;
use defuse_nep245::{MtBurnEvent, MtEvent, MtMintEvent};
use itertools::{Itertools, izip};
use near_sdk::{
    AccountId, AccountIdRef, Gas, PromiseResult, env, json_types::U128, require, serde_json,
};
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

// #[near]
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

    /// Generic internal helper for resolving deposit refunds across all token standards (NEP-141, NEP-171, NEP-245).
    ///
    /// This function:
    /// 1. Takes parallel vectors of token IDs, deposited amounts, and requested refunds
    /// 2. Checks available balance for each token in the receiver's account
    /// 3. Handles duplicate token IDs correctly by tracking planned withdrawals
    /// 4. Caps refunds at both the deposited amount and available balance
    /// 5. Performs a single batched withdrawal for all refunded tokens
    /// 6. Returns the actual refund amounts for each request
    pub fn resolve_deposit_internal(
        &mut self,
        receiver_id: &AccountId,
        token_ids: Vec<TokenId>,
        deposited_amounts: Vec<u128>,
    ) -> Vec<U128> {
        require!(
            env::predecessor_account_id() == *CURRENT_ACCOUNT_ID,
            "only self"
        );
        let tokens_count = token_ids.len();

        assert!(
            (tokens_count == deposited_amounts.len()),
            "token_ids and amounts must have the same length"
        );

        assert!(token_ids.iter().all_unique(), "token_ids must be unique");

        let requested_refunds = match env::promise_result(0) {
            PromiseResult::Successful(value) => serde_json::from_slice::<Vec<U128>>(&value)
                .ok()
                .filter(|refunds| refunds.len() == tokens_count)
                .map_or_else(
                    || deposited_amounts.clone(),
                    |refunds| refunds.into_iter().map(|elem| elem.0).collect(),
                ),
            // Do not refund on failure; rely solely on mt_on_transfer return values.
            // This aligns with NEP-141/171 behavior: if the receiver panics, no refund occurs.
            PromiseResult::Failed => deposited_amounts.clone(),
        };

        let actual_refunds = izip!(token_ids, deposited_amounts, &requested_refunds)
            .map(|(token, deposited, refund)| {
                let available = self
                    .accounts
                    .get(receiver_id.as_ref())
                    .map_or(0, |account| {
                        account
                            .as_inner_unchecked()
                            .token_balances
                            .amount_for(&token)
                    });
                (token, available.min(deposited.min(*refund)))
            })
            .collect::<Vec<_>>();

        if !requested_refunds.is_empty() {
            self.withdraw(
                receiver_id.as_ref(),
                actual_refunds.clone(),
                Some("refund unused tokens"),
                false,
            )
            .unwrap_or_default();
        }

        actual_refunds
            .into_iter()
            .map(|(_, amount)| amount.into())
            .collect()
    }
}

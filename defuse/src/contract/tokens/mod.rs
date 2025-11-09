mod nep141;
mod nep171;
mod nep245;

use super::Contract;
use defuse_core::{DefuseError, Result, token_id::TokenId};
use defuse_nep245::{MtBurnEvent, MtEvent, MtMintEvent};
use near_sdk::{AccountId, AccountIdRef, Gas, json_types::U128};
use std::{borrow::Cow, collections::HashMap};

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

    /// Generic internal helper for resolving deposit refunds across all token standards (NEP-141, NEP-171, NEP-245).
    ///
    /// This function:
    /// 1. Takes parallel vectors of token IDs, deposited amounts, and requested refunds
    /// 2. Checks available balance for each token in the receiver's account
    /// 3. Handles duplicate token IDs correctly by tracking planned withdrawals
    /// 4. Caps refunds at both the deposited amount and available balance
    /// 5. Performs a single batched withdrawal for all refunded tokens
    /// 6. Returns the actual refund amounts for each request
    pub(crate) fn resolve_deposit_internal(
        &mut self,
        receiver_id: &AccountId,
        token_ids: Vec<TokenId>,
        deposited_amounts: Vec<u128>,
        requested_refunds: Vec<u128>,
    ) -> Vec<u128> {
        let token_count = token_ids.len();
        let mut actual_refunds = vec![0u128; token_count];
        let mut planned_withdrawals: HashMap<TokenId, u128> = HashMap::new();

        for idx in 0..token_count {
            let requested_refund = requested_refunds[idx];
            if requested_refund == 0 {
                continue;
            }

            let token_id = &token_ids[idx];
            let deposited_amount = deposited_amounts[idx];

            let available = self
                .accounts
                .get(receiver_id.as_ref())
                .map(|account| {
                    account
                        .as_inner_unchecked()
                        .token_balances
                        .amount_for(token_id)
                })
                .unwrap_or(0);

            let already_planned = planned_withdrawals
                .get(token_id)
                .copied()
                .unwrap_or(0);
            let remaining = available.saturating_sub(already_planned);

            let refund_amount = requested_refund
                .min(deposited_amount)
                .min(remaining);

            if refund_amount > 0 {
                actual_refunds[idx] = refund_amount;
                planned_withdrawals
                    .entry(token_id.clone())
                    .and_modify(|planned| *planned += refund_amount)
                    .or_insert(refund_amount);
            }
        }

        if !planned_withdrawals.is_empty() {
            self.withdraw(
                receiver_id.as_ref(),
                planned_withdrawals.into_iter(),
                Some("refund unused tokens"),
                false,
            )
            .unwrap_or_default();
        }

        actual_refunds
    }
}

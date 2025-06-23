use crate::contract::{Contract, ContractExt};
use defuse_core::{DefuseError, Result, engine::StateView, token_id::TokenId};
use defuse_near_utils::{CURRENT_ACCOUNT_ID, UnwrapOrPanic, UnwrapOrPanicError};
use defuse_nep245::{MtEvent, MtTransferEvent, MultiTokenCore, receiver::ext_mt_receiver};
use near_plugins::{Pausable, pause};
use near_sdk::{
    AccountId, AccountIdRef, Gas, PromiseOrValue, assert_one_yocto, json_types::U128, near, require,
};
use std::borrow::Cow;

#[near]
impl MultiTokenCore for Contract {
    #[payable]
    fn mt_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: defuse_nep245::TokenId,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
    ) {
        self.mt_batch_transfer(
            receiver_id,
            [token_id].into(),
            [amount].into(),
            approval.map(|a| vec![Some(a)]),
            memo,
        );
    }

    #[pause(name = "mt_transfer")]
    #[payable]
    fn mt_batch_transfer(
        &mut self,
        receiver_id: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<(AccountId, u64)>>>,
        memo: Option<String>,
    ) {
        assert_one_yocto();
        require!(approvals.is_none(), "approvals are not supported");

        self.internal_mt_batch_transfer(
            self.ensure_auth_predecessor_id(),
            &receiver_id,
            &token_ids,
            &amounts,
            memo.as_deref(),
            false,
        )
        .unwrap_or_panic()
    }

    #[pause(name = "mt_transfer")]
    #[payable]
    fn mt_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: defuse_nep245::TokenId,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        self.mt_batch_transfer_call(
            receiver_id,
            [token_id].into(),
            [amount].into(),
            approval.map(|a| vec![Some(a)]),
            memo,
            msg,
        )
    }

    #[pause(name = "mt_transfer")]
    #[payable]
    fn mt_batch_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<(AccountId, u64)>>>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        assert_one_yocto();
        require!(approvals.is_none(), "approvals are not supported");

        self.internal_mt_batch_transfer_call(
            self.ensure_auth_predecessor_id().clone(),
            receiver_id,
            token_ids,
            amounts,
            memo.as_deref(),
            msg,
            false,
        )
        .unwrap_or_panic()
    }

    fn mt_token(
        &self,
        token_ids: Vec<defuse_nep245::TokenId>,
    ) -> Vec<Option<defuse_nep245::Token>> {
        token_ids
            .into_iter()
            .map(|token_id| {
                self.total_supplies
                    .contains_key(&token_id.parse().ok()?)
                    .then_some(defuse_nep245::Token {
                        token_id,
                        owner_id: None,
                    })
            })
            .collect()
    }

    fn mt_balance_of(&self, account_id: AccountId, token_id: defuse_nep245::TokenId) -> U128 {
        U128(self.internal_mt_balance_of(&account_id, &token_id))
    }

    fn mt_batch_balance_of(
        &self,
        account_id: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
    ) -> Vec<U128> {
        token_ids
            .into_iter()
            .map(|token_id| self.internal_mt_balance_of(&account_id, &token_id))
            .map(U128)
            .collect()
    }

    fn mt_supply(&self, token_id: defuse_nep245::TokenId) -> Option<U128> {
        Some(U128(
            self.total_supplies.amount_for(&token_id.parse().ok()?),
        ))
    }

    fn mt_batch_supply(&self, token_ids: Vec<defuse_nep245::TokenId>) -> Vec<Option<U128>> {
        token_ids
            .into_iter()
            .map(|token_id| self.mt_supply(token_id))
            .collect()
    }
}

impl Contract {
    pub(crate) fn internal_mt_balance_of(
        &self,
        account_id: &AccountIdRef,
        token_id: &defuse_nep245::TokenId,
    ) -> u128 {
        let Ok(token_id) = token_id.parse() else {
            return 0;
        };
        self.balance_of(account_id, &token_id)
    }

    pub(crate) fn internal_mt_batch_transfer(
        &mut self,
        sender_id: &AccountIdRef,
        receiver_id: &AccountIdRef,
        token_ids: &[defuse_nep245::TokenId],
        amounts: &[U128],
        memo: Option<&str>,
        force: bool,
    ) -> Result<()> {
        if sender_id == receiver_id || token_ids.len() != amounts.len() || amounts.is_empty() {
            return Err(DefuseError::InvalidIntent);
        }

        for (token_id, amount) in token_ids.iter().zip(amounts.iter().map(|a| a.0)) {
            if amount == 0 {
                return Err(DefuseError::InvalidIntent);
            }
            let token_id: TokenId = token_id.parse()?;

            self.accounts
                .get_mut(sender_id)
                .ok_or_else(|| DefuseError::AccountNotFound(sender_id.to_owned()))?
                .get_mut_maybe_forced(force)
                .ok_or_else(|| DefuseError::AccountLocked(sender_id.to_owned()))?
                .token_balances
                .sub(token_id.clone(), amount)
                .ok_or(DefuseError::BalanceOverflow)?;
            self.accounts
                .get_or_create(receiver_id.to_owned())
                // locked accounts are allowed to receive incoming transfers
                .as_inner_unchecked_mut()
                .token_balances
                .add(token_id, amount)
                .ok_or(DefuseError::BalanceOverflow)?;
        }

        MtEvent::MtTransfer(
            [MtTransferEvent {
                authorized_id: None,
                old_owner_id: sender_id.into(),
                new_owner_id: Cow::Borrowed(receiver_id),
                token_ids: token_ids.into(),
                amounts: amounts.into(),
                memo: memo.map(Into::into),
            }]
            .as_slice()
            .into(),
        )
        .emit();

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn internal_mt_batch_transfer_call(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        memo: Option<&str>,
        msg: String,
        force: bool,
    ) -> Result<PromiseOrValue<Vec<U128>>> {
        self.internal_mt_batch_transfer(
            &sender_id,
            &receiver_id,
            &token_ids,
            &amounts,
            memo,
            force,
        )?;

        let previous_owner_ids = vec![sender_id.clone(); token_ids.len()];

        Ok(ext_mt_receiver::ext(receiver_id.clone())
            .mt_on_transfer(
                sender_id,
                previous_owner_ids.clone(),
                token_ids.clone(),
                amounts.clone(),
                msg,
            )
            .then(
                Self::ext(CURRENT_ACCOUNT_ID.clone())
                    .with_static_gas(Self::mt_resolve_gas(token_ids.len()))
                    // do not distribute remaining gas here (so that all that's left goes to `mt_on_transfer`)
                    .with_unused_gas_weight(0)
                    .mt_resolve_transfer(previous_owner_ids, receiver_id, token_ids, amounts, None),
            )
            .into())
    }

    #[must_use]
    fn mt_resolve_gas(token_count: usize) -> Gas {
        // These represent a linear model total_gas_cost = per_token*n + base,
        // where `n` is the number of tokens.
        const MT_RESOLVE_TRANSFER_PER_TOKEN_GAS: Gas = Gas::from_tgas(2);
        const MT_RESOLVE_TRANSFER_BASE_GAS: Gas = Gas::from_tgas(8);
        let token_count: u64 = token_count.try_into().unwrap_or_panic_display();

        MT_RESOLVE_TRANSFER_BASE_GAS
            .checked_add(
                MT_RESOLVE_TRANSFER_PER_TOKEN_GAS
                    .checked_mul(token_count)
                    .unwrap_or_panic(),
            )
            .unwrap_or_panic()
    }
}

use defuse_core::{engine::StateView, tokens::TokenId, DefuseError, Result};
use defuse_near_utils::{UnwrapOrPanic, CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID};
use defuse_nep245::{receiver::ext_mt_receiver, MtEvent, MtTransferEvent, MultiTokenCore};
use near_plugins::{pause, Pausable};
use near_sdk::{
    assert_one_yocto, json_types::U128, near, require, AccountId, AccountIdRef, PromiseOrValue,
};

use crate::contract::{Contract, ContractExt};

use super::resolver::MT_RESOLVE_TRANSFER_GAS;

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
            &PREDECESSOR_ACCOUNT_ID,
            receiver_id,
            token_ids,
            amounts,
            memo.as_deref(),
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
            PREDECESSOR_ACCOUNT_ID.clone(),
            receiver_id,
            token_ids,
            amounts,
            memo.as_deref(),
            msg,
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
        receiver_id: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        memo: Option<&str>,
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
                .ok_or(DefuseError::AccountNotFound)?
                .token_balances
                .sub(token_id.clone(), amount)
                .ok_or(DefuseError::BalanceOverflow)?;
            self.accounts
                .get_or_create(receiver_id.clone())
                .token_balances
                .add(token_id, amount)
                .ok_or(DefuseError::BalanceOverflow)?;
        }

        MtEvent::MtTransfer(
            [MtTransferEvent {
                authorized_id: None,
                old_owner_id: sender_id.into(),
                new_owner_id: receiver_id.into(),
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

    pub(crate) fn internal_mt_batch_transfer_call(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        memo: Option<&str>,
        msg: String,
    ) -> Result<PromiseOrValue<Vec<U128>>> {
        self.internal_mt_batch_transfer(
            &sender_id,
            receiver_id.clone(),
            token_ids.clone(),
            amounts.clone(),
            memo,
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
                Contract::ext(CURRENT_ACCOUNT_ID.clone())
                    .with_static_gas(MT_RESOLVE_TRANSFER_GAS)
                    .mt_resolve_transfer(previous_owner_ids, receiver_id, token_ids, amounts, None),
            )
            .into())
    }
}

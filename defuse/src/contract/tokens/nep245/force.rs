#![allow(clippy::too_many_arguments)]

use defuse_near_utils::UnwrapOrPanic;
use defuse_nep245::TokenId;
use near_plugins::{AccessControllable, access_control_any};
use near_sdk::{AccountId, PromiseOrValue, assert_one_yocto, json_types::U128, near, require};

use crate::{
    contract::{Contract, ContractExt, Role},
    tokens::nep245::MultiTokenForcedCore,
};

#[near]
impl MultiTokenForcedCore for Contract {
    #[access_control_any(roles(Role::DAO, Role::UnrestrictedWithdrawer))]
    #[payable]
    fn mt_force_transfer(
        &mut self,
        owner_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
    ) {
        self.mt_force_batch_transfer(
            owner_id,
            receiver_id,
            [token_id].into(),
            [amount].into(),
            approval.map(|a| vec![Some(a)]),
            memo,
        );
    }

    #[access_control_any(roles(Role::DAO, Role::UnrestrictedWithdrawer))]
    #[payable]
    fn mt_force_batch_transfer(
        &mut self,
        owner_id: AccountId,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<(AccountId, u64)>>>,
        memo: Option<String>,
    ) {
        assert_one_yocto();
        require!(approvals.is_none(), "approvals are not supported");

        self.internal_mt_batch_transfer(
            &owner_id,
            &receiver_id,
            &token_ids,
            &amounts,
            memo.as_deref(),
            true,
        )
        .unwrap_or_panic()
    }

    #[access_control_any(roles(Role::DAO, Role::UnrestrictedWithdrawer))]
    #[payable]
    fn mt_force_transfer_call(
        &mut self,
        owner_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        self.mt_force_batch_transfer_call(
            owner_id,
            receiver_id,
            [token_id].into(),
            [amount].into(),
            approval.map(|a| vec![Some(a)]),
            memo,
            msg,
        )
    }

    #[access_control_any(roles(Role::DAO, Role::UnrestrictedWithdrawer))]
    #[payable]
    fn mt_force_batch_transfer_call(
        &mut self,
        owner_id: AccountId,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<(AccountId, u64)>>>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        assert_one_yocto();
        require!(approvals.is_none(), "approvals are not supported");

        self.internal_mt_batch_transfer_call(
            owner_id,
            receiver_id,
            token_ids,
            amounts,
            memo.as_deref(),
            msg,
            true,
        )
        .unwrap_or_panic()
    }
}

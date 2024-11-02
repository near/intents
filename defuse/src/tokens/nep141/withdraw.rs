use defuse_contracts::{
    defuse::{
        intents::tokens::FtWithdraw,
        tokens::{
            nep141::{FungibleTokenWithdrawResolver, FungibleTokenWithdrawer},
            TokenId,
        },
        DefuseError, Result,
    },
    utils::{
        cache::{CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID},
        PanicError, UnwrapOrPanic, UnwrapOrPanicError,
    },
};
use near_contract_standards::storage_management::ext_storage_management;
use near_plugins::{pause, Pausable};
use near_sdk::{
    assert_one_yocto, env,
    json_types::U128,
    near, require,
    serde_json::{self, json},
    AccountId, Gas, NearToken, Promise, PromiseOrValue, PromiseResult,
};

use crate::{
    accounts::Account, state::State, tokens::storage_management::STORAGE_DEPOSIT_GAS, DefuseImpl,
    DefuseImplExt,
};

const FT_TRANSFER_GAS: Gas = Gas::from_tgas(10);

#[near]
impl FungibleTokenWithdrawer for DefuseImpl {
    #[pause]
    #[payable]
    fn ft_withdraw(
        &mut self,
        token: AccountId,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
    ) -> PromiseOrValue<bool> {
        assert_one_yocto();
        self.do_ft_withdraw(
            PREDECESSOR_ACCOUNT_ID.clone(),
            FtWithdraw {
                token,
                receiver_id,
                amount,
                memo,
                storage_deposit: None,
            },
        )
    }
}

impl State {
    pub fn ft_withdraw(
        &mut self,
        sender_id: AccountId,
        sender: &mut Account,
        withdraw @ FtWithdraw {
            storage_deposit, ..
        }: FtWithdraw,
    ) -> Result<PromiseOrValue<bool>> {
        if let Some(storage_deposit) = storage_deposit {
            // check amount before unwrapping wNEAR
            require!(withdraw.amount.0 > 0, "zero amount");
            Ok(self
                .unwrap_wnear(
                    sender_id.clone(),
                    sender,
                    storage_deposit,
                    Some("storage_deposit"),
                )?
                .then(
                    DefuseImpl::ext(CURRENT_ACCOUNT_ID.clone())
                        .with_static_gas(DefuseImpl::DO_FT_WITHDRAW_GAS)
                        .do_ft_withdraw(sender_id, withdraw),
                )
                .into())
        } else {
            self.do_ft_withdraw(sender_id, sender, withdraw)
        }
    }
}

#[near]
impl DefuseImpl {
    pub(crate) const FT_RESOLVE_WITHDRAW_GAS: Gas = Gas::from_tgas(5);
    const DO_FT_WITHDRAW_GAS: Gas = Gas::from_tgas(5)
        // do_ft_withdraw() method is called only with storage_deposit
        .saturating_add(STORAGE_DEPOSIT_GAS)
        .saturating_add(FT_TRANSFER_GAS)
        .saturating_add(Self::FT_RESOLVE_WITHDRAW_GAS);

    #[private]
    pub fn do_ft_withdraw(
        &mut self,
        sender_id: AccountId,
        withdraw: FtWithdraw,
    ) -> PromiseOrValue<bool> {
        if withdraw.storage_deposit.is_some() {
            let PromiseResult::Successful(data) = env::promise_result(0) else {
                "near_withdraw failed".panic_static_str();
            };
            let near_withdraw_ok: bool = serde_json::from_slice(&data).unwrap_or_panic_display();
            require!(near_withdraw_ok, "near_withdraw failed");
        }

        let sender = self
            .accounts
            .get_mut(&sender_id)
            .ok_or(DefuseError::AccountNotFound)
            .unwrap_or_panic();
        self.state
            .do_ft_withdraw(sender_id, sender, withdraw)
            .unwrap_or_panic()
    }
}

impl State {
    fn do_ft_withdraw(
        &mut self,
        sender_id: AccountId,
        sender: &mut Account,
        FtWithdraw {
            token,
            receiver_id,
            amount,
            memo,
            storage_deposit,
        }: FtWithdraw,
    ) -> Result<PromiseOrValue<bool>> {
        self.internal_withdraw(
            &sender_id,
            sender,
            [(TokenId::Nep141(token.clone()), amount.0)],
            Some("withdraw"),
        )?;

        Ok(if let Some(storage_deposit) = storage_deposit {
            ext_storage_management::ext(token.clone())
                .with_attached_deposit(storage_deposit)
                .with_static_gas(STORAGE_DEPOSIT_GAS)
                .storage_deposit(Some(receiver_id.clone()), None)
        } else {
            Promise::new(token.clone())
        }
        .ft_transfer(&receiver_id, amount.0, memo.as_deref())
        .then(
            DefuseImpl::ext(CURRENT_ACCOUNT_ID.clone())
                .with_static_gas(DefuseImpl::FT_RESOLVE_WITHDRAW_GAS)
                .ft_resolve_withdraw(token, sender_id, amount),
        )
        .into())
    }
}

#[near]
impl FungibleTokenWithdrawResolver for DefuseImpl {
    #[private]
    fn ft_resolve_withdraw(
        &mut self,
        token: AccountId,
        sender_id: AccountId,
        amount: U128,
    ) -> bool {
        let ok =
            matches!(env::promise_result(0), PromiseResult::Successful(data) if data.is_empty());

        if !ok {
            self.internal_deposit(
                sender_id,
                [(TokenId::Nep141(token), amount.0)],
                Some("refund"),
            )
            .unwrap_or_panic();
        }

        ok
    }
}

pub trait FtExt {
    fn ft_transfer(self, receiver_id: &AccountId, amount: u128, memo: Option<&str>) -> Self;
}

impl FtExt for Promise {
    fn ft_transfer(self, receiver_id: &AccountId, amount: u128, memo: Option<&str>) -> Self {
        self.function_call(
            "ft_transfer".to_string(),
            serde_json::to_vec(&json!({
                "receiver_id": receiver_id,
                "amount": U128(amount),
                "memo": memo,
            }))
            .unwrap_or_panic_display(),
            NearToken::from_yoctonear(1),
            FT_TRANSFER_GAS,
        )
    }
}

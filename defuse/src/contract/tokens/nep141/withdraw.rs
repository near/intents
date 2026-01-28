use crate::{
    contract::{
        Contract, ContractExt, Role,
        tokens::{RefundLogCheck, STORAGE_DEPOSIT_GAS},
    },
    tokens::nep141::{
        FungibleTokenForceWithdrawer, FungibleTokenWithdrawResolver, FungibleTokenWithdrawer,
    },
};
use core::iter;
use defuse_core::{
    DefuseError, Result, engine::StateView, intents::tokens::FtWithdraw,
    token_id::nep141::Nep141TokenId,
};
use defuse_near_utils::{REFUND_MEMO, UnwrapOrPanic};
use defuse_wnear::{NEAR_WITHDRAW_GAS, ext_wnear};
use near_contract_standards::{
    fungible_token::core::ext_ft_core, storage_management::ext_storage_management,
};
use near_plugins::{AccessControllable, Pausable, access_control_any, pause};
use near_sdk::{
    AccountId, Gas, NearToken, Promise, PromiseOrValue, assert_one_yocto, env, json_types::U128,
    near, require, serde_json,
};

#[near]
impl FungibleTokenWithdrawer for Contract {
    #[pause]
    #[payable]
    fn ft_withdraw(
        &mut self,
        token: AccountId,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> PromiseOrValue<U128> {
        assert_one_yocto();
        self.internal_ft_withdraw(
            self.ensure_auth_predecessor_id(),
            FtWithdraw {
                token,
                receiver_id,
                amount,
                memo,
                msg,
                storage_deposit: None,
                min_gas: None,
            },
            false,
        )
        .unwrap_or_panic()
    }
}

impl Contract {
    pub(crate) fn internal_ft_withdraw(
        &mut self,
        owner_id: AccountId,
        withdraw: FtWithdraw,
        force: bool,
    ) -> Result<PromiseOrValue<U128>> {
        self.withdraw(
            &owner_id,
            iter::once((
                Nep141TokenId::new(withdraw.token.clone()).into(),
                withdraw.amount.0,
            ))
            .chain(withdraw.storage_deposit.map(|amount| {
                (
                    Nep141TokenId::new(self.wnear_id().into_owned()).into(),
                    amount.as_yoctonear(),
                )
            })),
            Some("withdraw"),
            force,
            RefundLogCheck::CheckRefundLogLength,
        )?;

        let is_call = withdraw.is_call();
        Ok(if let Some(storage_deposit) = withdraw.storage_deposit {
            ext_wnear::ext(self.wnear_id.clone())
                .with_attached_deposit(NearToken::from_yoctonear(1))
                .with_static_gas(NEAR_WITHDRAW_GAS)
                // do not distribute remaining gas here
                .with_unused_gas_weight(0)
                .near_withdraw(U128(storage_deposit.as_yoctonear()))
                .then(
                    // schedule storage_deposit() only after near_withdraw() returns
                    Self::ext(env::current_account_id())
                        .with_static_gas(
                            Self::DO_FT_WITHDRAW_GAS
                                .checked_add(withdraw.min_gas())
                                .ok_or(DefuseError::GasOverflow)
                                .unwrap_or_panic(),
                        )
                        .do_ft_withdraw(withdraw.clone()),
                )
        } else {
            Self::do_ft_withdraw(withdraw.clone())
        }
        .then(
            Self::ext(env::current_account_id())
                .with_static_gas(Self::FT_RESOLVE_WITHDRAW_GAS)
                // do not distribute remaining gas here
                .with_unused_gas_weight(0)
                .ft_resolve_withdraw(withdraw.token, owner_id, withdraw.amount, is_call),
        )
        .into())
    }
}

#[near]
impl Contract {
    const FT_RESOLVE_WITHDRAW_GAS: Gas = Gas::from_tgas(5);
    const DO_FT_WITHDRAW_GAS: Gas = Gas::from_tgas(5)
        // do_ft_withdraw() method is called externally
        // only with storage_deposit
        .saturating_add(STORAGE_DEPOSIT_GAS);

    #[private]
    pub fn do_ft_withdraw(withdraw: FtWithdraw) -> Promise {
        let min_gas = withdraw.min_gas();
        let p = if let Some(storage_deposit) = withdraw.storage_deposit {
            require!(
                matches!(env::promise_result_checked(0, 0), Ok(data) if data.is_empty()),
                "near_withdraw failed",
            );

            ext_storage_management::ext(withdraw.token)
                .with_attached_deposit(storage_deposit)
                .with_static_gas(STORAGE_DEPOSIT_GAS)
                // do not distribute remaining gas here
                .with_unused_gas_weight(0)
                .storage_deposit(Some(withdraw.receiver_id.clone()), None)
        } else {
            Promise::new(withdraw.token)
        };

        let p = ext_ft_core::ext_on(p)
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(min_gas)
            // distribute remaining gas here
            .with_unused_gas_weight(1);
        if let Some(msg) = withdraw.msg {
            p.ft_transfer_call(withdraw.receiver_id, withdraw.amount, withdraw.memo, msg)
        } else {
            p.ft_transfer(withdraw.receiver_id, withdraw.amount, withdraw.memo)
        }
    }
}

#[near]
impl FungibleTokenWithdrawResolver for Contract {
    #[private]
    fn ft_resolve_withdraw(
        &mut self,
        token: AccountId,
        sender_id: AccountId,
        amount: U128,
        is_call: bool,
    ) -> U128 {
        const MAX_RESULT_LENGTH: usize = "\"+340282366920938463463374607431768211455\"".len(); // u128::MAX

        let used = env::promise_result_checked(0, MAX_RESULT_LENGTH).map_or(
            if is_call {
                // do not refund on failed `ft_transfer_call` due to
                // NEP-141 vulnerability: `ft_resolve_transfer` fails to
                // read result of `ft_on_transfer` due to insufficient gas
                amount.0
            } else {
                0
            },
            |value| {
                if is_call {
                    // `ft_transfer_call` returns successfully transferred amount
                    serde_json::from_slice::<U128>(&value)
                        .unwrap_or_default()
                        .0
                        .min(amount.0)
                } else if value.is_empty() {
                    // `ft_transfer` returns empty result on success
                    amount.0
                } else {
                    0
                }
            },
        );

        let refund = amount.0.saturating_sub(used);
        if refund > 0 {
            self.deposit(
                sender_id,
                [(Nep141TokenId::new(token).into(), refund)],
                Some(REFUND_MEMO),
                RefundLogCheck::Unchecked,
            )
            .unwrap_or_panic();
        }

        U128(used)
    }
}

#[near]
impl FungibleTokenForceWithdrawer for Contract {
    #[access_control_any(roles(Role::DAO, Role::UnrestrictedWithdrawer))]
    #[payable]
    fn ft_force_withdraw(
        &mut self,
        owner_id: AccountId,
        token: AccountId,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> PromiseOrValue<U128> {
        assert_one_yocto();
        self.internal_ft_withdraw(
            owner_id,
            FtWithdraw {
                token,
                receiver_id,
                amount,
                memo,
                msg,
                storage_deposit: None,
                min_gas: None,
            },
            true,
        )
        .unwrap_or_panic()
    }
}

use core::iter;

use defuse_core::{Result, engine::StateView, intents::tokens::FtWithdraw, tokens::TokenId};
use defuse_near_utils::{
    CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic, UnwrapOrPanicError,
};

use defuse_wnear::{NEAR_WITHDRAW_GAS, ext_wnear};
use near_contract_standards::storage_management::ext_storage_management;
use near_plugins::{AccessControllable, Pausable, access_control_any, pause};
use near_sdk::{
    AccountId, Gas, NearToken, Promise, PromiseOrValue, PromiseResult, assert_one_yocto, env,
    json_types::U128,
    near, require,
    serde_json::{self, json},
};

use crate::{
    contract::{Contract, ContractExt, Role, tokens::STORAGE_DEPOSIT_GAS},
    tokens::nep141::{
        FungibleTokenForceWithdrawer, FungibleTokenWithdrawResolver, FungibleTokenWithdrawer,
    },
};

const FT_TRANSFER_GAS: Gas = Gas::from_tgas(15);
const FT_TRANSFER_CALL_GAS: Gas = Gas::from_tgas(50);

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
            PREDECESSOR_ACCOUNT_ID.clone(),
            FtWithdraw {
                token,
                receiver_id,
                amount,
                memo,
                msg,
                storage_deposit: None,
            },
        )
        .unwrap_or_panic()
    }
}

impl Contract {
    pub(crate) fn internal_ft_withdraw(
        &mut self,
        owner_id: AccountId,
        withdraw: FtWithdraw,
    ) -> Result<PromiseOrValue<U128>> {
        self.withdraw(
            &owner_id,
            iter::once((TokenId::Nep141(withdraw.token.clone()), withdraw.amount.0)).chain(
                withdraw.storage_deposit.map(|amount| {
                    (
                        TokenId::Nep141(self.wnear_id().into_owned()),
                        amount.as_yoctonear(),
                    )
                }),
            ),
            Some("withdraw"),
        )?;

        let is_call = withdraw.msg.is_some();
        Ok(if let Some(storage_deposit) = withdraw.storage_deposit {
            ext_wnear::ext(self.wnear_id.clone())
                .with_attached_deposit(NearToken::from_yoctonear(1))
                .with_static_gas(NEAR_WITHDRAW_GAS)
                .near_withdraw(U128(storage_deposit.as_yoctonear()))
                .then(
                    // schedule storage_deposit() only after near_withdraw() returns
                    Self::ext(CURRENT_ACCOUNT_ID.clone())
                        .with_static_gas(Self::DO_FT_WITHDRAW_GAS.saturating_add(if is_call {
                            FT_TRANSFER_CALL_GAS
                        } else {
                            FT_TRANSFER_GAS
                        }))
                        .do_ft_withdraw(withdraw.clone()),
                )
        } else {
            Self::do_ft_withdraw(withdraw.clone())
        }
        .then(
            Self::ext(CURRENT_ACCOUNT_ID.clone())
                .with_static_gas(Self::FT_RESOLVE_WITHDRAW_GAS)
                .ft_resolve_withdraw(withdraw.token, owner_id, withdraw.amount, is_call),
        )
        .into())
    }
}

#[near]
impl Contract {
    const FT_RESOLVE_WITHDRAW_GAS: Gas = Gas::from_tgas(5);
    const DO_FT_WITHDRAW_GAS: Gas = Gas::from_tgas(3)
        // do_ft_withdraw() method is called externally
        // only with storage_deposit
        .saturating_add(STORAGE_DEPOSIT_GAS);

    #[must_use]
    #[private]
    pub fn do_ft_withdraw(withdraw: FtWithdraw) -> Promise {
        let p = if let Some(storage_deposit) = withdraw.storage_deposit {
            require!(
                matches!(env::promise_result(0), PromiseResult::Successful(data) if data.is_empty()),
                "near_withdraw failed",
            );

            ext_storage_management::ext(withdraw.token)
                .with_attached_deposit(storage_deposit)
                .with_static_gas(STORAGE_DEPOSIT_GAS)
                .storage_deposit(Some(withdraw.receiver_id.clone()), None)
        } else {
            Promise::new(withdraw.token)
        };

        if let Some(msg) = withdraw.msg.as_deref() {
            p.ft_transfer_call(
                &withdraw.receiver_id,
                withdraw.amount.0,
                withdraw.memo.as_deref(),
                msg,
            )
        } else {
            p.ft_transfer(
                &withdraw.receiver_id,
                withdraw.amount.0,
                withdraw.memo.as_deref(),
            )
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
        let used = match env::promise_result(0) {
            PromiseResult::Successful(value) => {
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
            }
            PromiseResult::Failed => {
                if is_call {
                    // do not refund on failed `ft_transfer_call` due to
                    // NEP-141 vulnerability: `ft_resolve_transfer` fails to
                    // read result of `ft_on_transfer` due to insufficient gas
                    amount.0
                } else {
                    0
                }
            }
        };

        let refund = amount.0.saturating_sub(used);
        if refund > 0 {
            self.deposit(
                sender_id,
                [(TokenId::Nep141(token), refund)],
                Some("refund"),
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
            },
        )
        .unwrap_or_panic()
    }
}

pub trait FtExt {
    fn ft_transfer(self, receiver_id: &AccountId, amount: u128, memo: Option<&str>) -> Self;
    fn ft_transfer_call(
        self,
        receiver_id: &AccountId,
        amount: u128,
        memo: Option<&str>,
        msg: &str,
    ) -> Self;
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

    fn ft_transfer_call(
        self,
        receiver_id: &AccountId,
        amount: u128,
        memo: Option<&str>,
        msg: &str,
    ) -> Self {
        self.function_call(
            "ft_transfer_call".to_string(),
            serde_json::to_vec(&json!({
                "receiver_id": receiver_id,
                "amount": U128(amount),
                "memo": memo,
                "msg": msg,
            }))
            .unwrap_or_panic_display(),
            NearToken::from_yoctonear(1),
            FT_TRANSFER_CALL_GAS,
        )
    }
}

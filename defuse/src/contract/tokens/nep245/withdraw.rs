#![allow(clippy::too_many_arguments)]

use crate::{
    contract::{Contract, ContractExt, Role, tokens::STORAGE_DEPOSIT_GAS},
    tokens::nep245::{
        MultiTokenForcedWithdrawer, MultiTokenWithdrawResolver, MultiTokenWithdrawer,
    },
};
use defuse_core::{
    DefuseError, Result,
    engine::StateView,
    intents::tokens::MtWithdraw,
    token_id::{nep141::Nep141TokenId, nep245::Nep245TokenId},
};
use defuse_near_utils::{MaxJsonLength, REFUND_MEMO, UnwrapOrPanic, UnwrapOrPanicError};
use defuse_nep245::ext_mt_core;
use defuse_wnear::{NEAR_WITHDRAW_GAS, ext_wnear};
use near_contract_standards::storage_management::ext_storage_management;
use near_plugins::{AccessControllable, Pausable, access_control_any, pause};
use near_sdk::{
    AccountId, Gas, NearToken, Promise, PromiseOrValue, assert_one_yocto, env, json_types::U128,
    near, require, serde_json,
};

#[near]
impl MultiTokenWithdrawer for Contract {
    #[pause]
    #[payable]
    fn mt_withdraw(
        &mut self,
        token: AccountId,
        receiver_id: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        memo: Option<String>,
        msg: Option<String>,
    ) -> PromiseOrValue<Vec<U128>> {
        assert_one_yocto();
        self.internal_mt_withdraw(
            self.ensure_auth_predecessor_id(),
            MtWithdraw {
                token,
                receiver_id,
                token_ids,
                amounts,
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
    pub(crate) fn internal_mt_withdraw(
        &mut self,
        owner_id: AccountId,
        withdraw: MtWithdraw,
        force: bool,
    ) -> Result<PromiseOrValue<Vec<U128>>> {
        if withdraw.token_ids.len() != withdraw.amounts.len() || withdraw.token_ids.is_empty() {
            return Err(DefuseError::InvalidIntent);
        }

        self.withdraw(
            &owner_id,
            withdraw
                .token_ids
                .iter()
                .cloned()
                .map(|token_id| Nep245TokenId::new(withdraw.token.clone(), token_id))
                .map(Into::into)
                .zip(withdraw.amounts.iter().map(|a| a.0))
                .chain(withdraw.storage_deposit.map(|amount| {
                    (
                        Nep141TokenId::new(self.wnear_id().into_owned()).into(),
                        amount.as_yoctonear(),
                    )
                })),
            Some("withdraw"),
            force,
        )?;

        let is_call = withdraw.msg.is_some();
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
                            Self::DO_MT_WITHDRAW_GAS
                                .checked_add(withdraw.min_gas())
                                .ok_or(DefuseError::GasOverflow)
                                .unwrap_or_panic(),
                        )
                        .do_mt_withdraw(withdraw.clone()),
                )
        } else {
            Self::do_mt_withdraw(withdraw.clone())
        }
        .then(
            Self::ext(env::current_account_id())
                .with_static_gas(Self::mt_resolve_withdraw_gas(withdraw.token_ids.len()))
                // do not distribute remaining gas here
                .with_unused_gas_weight(0)
                .mt_resolve_withdraw(
                    withdraw.token,
                    owner_id,
                    withdraw.token_ids,
                    withdraw.amounts,
                    is_call,
                ),
        )
        .into())
    }

    #[must_use]
    fn mt_resolve_withdraw_gas(token_count: usize) -> Gas {
        // Values chosen to be similar to `MT_RESOLVE_TRANSFER_*` values
        const MT_RESOLVE_WITHDRAW_PER_TOKEN_GAS: Gas = Gas::from_tgas(2);
        const MT_RESOLVE_WITHDRAW_BASE_GAS: Gas = Gas::from_tgas(8);

        let token_count: u64 = token_count.try_into().unwrap_or_panic_display();

        MT_RESOLVE_WITHDRAW_BASE_GAS
            .checked_add(
                MT_RESOLVE_WITHDRAW_PER_TOKEN_GAS
                    .checked_mul(token_count)
                    .unwrap_or_panic(),
            )
            .unwrap_or_panic()
    }
}

#[near]
impl Contract {
    const DO_MT_WITHDRAW_GAS: Gas = Gas::from_tgas(5)
        // do_nft_withdraw() method is called externally
        // only with storage_deposit
        .saturating_add(STORAGE_DEPOSIT_GAS);

    #[private]
    pub fn do_mt_withdraw(withdraw: MtWithdraw) -> Promise {
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

        let p = ext_mt_core::ext_on(p)
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(min_gas)
            // distribute remaining gas here
            .with_unused_gas_weight(1);
        if let Some(msg) = withdraw.msg {
            p.mt_batch_transfer_call(
                withdraw.receiver_id,
                withdraw.token_ids,
                withdraw.amounts,
                None,
                withdraw.memo,
                msg,
            )
        } else {
            p.mt_batch_transfer(
                withdraw.receiver_id,
                withdraw.token_ids,
                withdraw.amounts,
                None,
                withdraw.memo,
            )
        }
    }
}

#[near]
impl MultiTokenWithdrawResolver for Contract {
    #[private]
    fn mt_resolve_withdraw(
        &mut self,
        token: AccountId,
        sender_id: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        is_call: bool,
    ) -> Vec<U128> {
        require!(
            token_ids.len() == amounts.len() && !amounts.is_empty(),
            "invalid args"
        );

        let mut used = env::promise_result_checked(0, Vec::<U128>::max_json_length(amounts.len()))
            .map_or_else(
                |_err| {
                    if is_call {
                        // do not refund on failed `mt_batch_transfer_call` due to
                        // NEP-141 vulnerability: `mt_resolve_transfer` fails to
                        // read result of `mt_on_transfer` due to insufficient gas
                        amounts.clone()
                    } else {
                        vec![U128(0); amounts.len()]
                    }
                },
                |value| {
                    if is_call {
                        // `mt_batch_transfer_call` returns successfully transferred amounts
                        serde_json::from_slice::<Vec<U128>>(&value)
                            .ok()
                            .filter(|used| used.len() == amounts.len())
                            .unwrap_or_else(|| vec![U128(0); amounts.len()])
                    } else if value.is_empty() {
                        // `mt_batch_transfer` returns empty result on success
                        amounts.clone()
                    } else {
                        vec![U128(0); amounts.len()]
                    }
                },
            );

        self.deposit(
            sender_id,
            token_ids
                .into_iter()
                .zip(amounts)
                .zip(&mut used)
                .filter_map(|((token_id, amount), used)| {
                    // update min during iteration
                    used.0 = used.0.min(amount.0);
                    let refund = amount.0.saturating_sub(used.0);
                    if refund > 0 {
                        Some((Nep245TokenId::new(token.clone(), token_id).into(), refund))
                    } else {
                        None
                    }
                }),
            Some(REFUND_MEMO),
        )
        .unwrap_or_panic();

        used
    }
}

#[near]
impl MultiTokenForcedWithdrawer for Contract {
    #[access_control_any(roles(Role::DAO, Role::UnrestrictedWithdrawer))]
    #[payable]
    fn mt_force_withdraw(
        &mut self,
        owner_id: AccountId,
        token: AccountId,
        receiver_id: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        memo: Option<String>,
        msg: Option<String>,
    ) -> PromiseOrValue<Vec<U128>> {
        assert_one_yocto();
        self.internal_mt_withdraw(
            owner_id,
            MtWithdraw {
                token,
                receiver_id,
                token_ids,
                amounts,
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

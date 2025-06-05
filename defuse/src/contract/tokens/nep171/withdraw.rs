use std::iter;

use defuse_core::{
    DefuseError, Result, engine::StateView, intents::tokens::NftWithdraw, tokens::TokenId,
};
use defuse_near_utils::{
    CURRENT_ACCOUNT_ID, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic, UnwrapOrPanicError,
};
use defuse_wnear::{NEAR_WITHDRAW_GAS, ext_wnear};
use near_contract_standards::{non_fungible_token, storage_management::ext_storage_management};
use near_plugins::{AccessControllable, Pausable, access_control_any, pause};
use near_sdk::{
    AccountId, Gas, GasWeight, NearToken, Promise, PromiseOrValue, PromiseResult, assert_one_yocto,
    env,
    json_types::U128,
    near, require,
    serde_json::{self, json},
};

use crate::{
    contract::{Contract, ContractExt, Role, tokens::STORAGE_DEPOSIT_GAS},
    tokens::nep171::{
        NonFungibleTokenForceWithdrawer, NonFungibleTokenWithdrawResolver,
        NonFungibleTokenWithdrawer,
    },
};

#[near]
impl NonFungibleTokenWithdrawer for Contract {
    #[pause]
    #[payable]
    fn nft_withdraw(
        &mut self,
        token: AccountId,
        receiver_id: AccountId,
        token_id: non_fungible_token::TokenId,
        memo: Option<String>,
        msg: Option<String>,
    ) -> PromiseOrValue<bool> {
        assert_one_yocto();
        self.internal_nft_withdraw(
            PREDECESSOR_ACCOUNT_ID.clone(),
            NftWithdraw {
                token,
                receiver_id,
                token_id,
                memo,
                msg,
                storage_deposit: None,
                min_gas: None,
            },
        )
        .unwrap_or_panic()
    }
}

impl Contract {
    pub(crate) fn internal_nft_withdraw(
        &mut self,
        owner_id: AccountId,
        withdraw: NftWithdraw,
    ) -> Result<PromiseOrValue<bool>> {
        self.withdraw(
            &owner_id,
            iter::once((
                TokenId::Nep171(withdraw.token.clone(), withdraw.token_id.clone()),
                1,
            ))
            .chain(withdraw.storage_deposit.map(|amount| {
                (
                    TokenId::Nep141(self.wnear_id().into_owned()),
                    amount.as_yoctonear(),
                )
            })),
            Some("withdraw"),
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
                    Self::ext(CURRENT_ACCOUNT_ID.clone())
                        .with_static_gas(
                            Self::DO_NFT_WITHDRAW_GAS
                                .checked_add(withdraw.min_gas())
                                .ok_or(DefuseError::GasOverflow)
                                .unwrap_or_panic(),
                        )
                        .do_nft_withdraw(withdraw.clone()),
                )
        } else {
            Self::do_nft_withdraw(withdraw.clone())
        }
        .then(
            Self::ext(CURRENT_ACCOUNT_ID.clone())
                .with_static_gas(Self::NFT_RESOLVE_WITHDRAW_GAS)
                // do not distribute remaining gas here
                .with_unused_gas_weight(0)
                .nft_resolve_withdraw(withdraw.token, owner_id, withdraw.token_id, is_call),
        )
        .into())
    }
}

#[near]
impl Contract {
    const NFT_RESOLVE_WITHDRAW_GAS: Gas = Gas::from_tgas(5);
    const DO_NFT_WITHDRAW_GAS: Gas = Gas::from_tgas(3)
        // do_nft_withdraw() method is called externally
        // only with storage_deposit
        .saturating_add(STORAGE_DEPOSIT_GAS);

    #[must_use]
    #[private]
    pub fn do_nft_withdraw(withdraw: NftWithdraw) -> Promise {
        let min_gas = withdraw.min_gas();
        let p = if let Some(storage_deposit) = withdraw.storage_deposit {
            require!(
                matches!(env::promise_result(0), PromiseResult::Successful(data) if data.is_empty()),
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

        if let Some(msg) = withdraw.msg.as_deref() {
            p.nft_transfer_call(
                &withdraw.receiver_id,
                &withdraw.token_id,
                withdraw.memo.as_deref(),
                msg,
                min_gas,
            )
        } else {
            p.nft_transfer(
                &withdraw.receiver_id,
                &withdraw.token_id,
                withdraw.memo.as_deref(),
                min_gas,
            )
        }
    }
}

#[near]
impl NonFungibleTokenWithdrawResolver for Contract {
    #[private]
    fn nft_resolve_withdraw(
        &mut self,
        token: AccountId,
        sender_id: AccountId,
        token_id: non_fungible_token::TokenId,
        is_call: bool,
    ) -> bool {
        let used = match env::promise_result(0) {
            PromiseResult::Successful(value) => {
                if is_call {
                    // `nft_transfer_call` returns true if token was successfully transferred
                    serde_json::from_slice(&value).unwrap_or_default()
                } else {
                    // `nft_transfer` returns empty result on success
                    value.is_empty()
                }
            }
            // do not refund on failed `nft_transfer_call` due to
            // NEP-141 vulnerability: `nft_resolve_transfer` fails to
            // read result of `nft_on_transfer` due to insufficient gas
            PromiseResult::Failed => is_call,
        };

        if !used {
            self.deposit(
                sender_id,
                [(TokenId::Nep171(token, token_id), 1)],
                Some("refund"),
            )
            .unwrap_or_panic();
        }

        used
    }
}

#[near]
impl NonFungibleTokenForceWithdrawer for Contract {
    #[access_control_any(roles(Role::DAO, Role::UnrestrictedWithdrawer))]
    #[payable]
    fn nft_force_withdraw(
        &mut self,
        owner_id: AccountId,
        token: AccountId,
        receiver_id: AccountId,
        token_id: non_fungible_token::TokenId,
        memo: Option<String>,
        msg: Option<String>,
    ) -> PromiseOrValue<bool> {
        assert_one_yocto();
        self.internal_nft_withdraw(
            owner_id,
            NftWithdraw {
                token,
                receiver_id,
                token_id,
                memo,
                msg,
                storage_deposit: None,
                min_gas: None,
            },
        )
        .unwrap_or_panic()
    }
}

pub trait NftExt {
    fn nft_transfer(
        self,
        receiver_id: &AccountId,
        token_id: &non_fungible_token::TokenId,
        memo: Option<&str>,
        min_gas: Gas,
    ) -> Self;

    fn nft_transfer_call(
        self,
        receiver_id: &AccountId,
        token_id: &non_fungible_token::TokenId,
        memo: Option<&str>,
        msg: &str,
        min_gas: Gas,
    ) -> Self;
}

impl NftExt for Promise {
    fn nft_transfer(
        self,
        receiver_id: &AccountId,
        token_id: &non_fungible_token::TokenId,
        memo: Option<&str>,
        min_gas: Gas,
    ) -> Self {
        self.function_call_weight(
            "nft_transfer".to_string(),
            serde_json::to_vec(&json!({
                "receiver_id": receiver_id,
                "token_id": token_id,
                "memo": memo,
            }))
            .unwrap_or_panic_display(),
            NearToken::from_yoctonear(1),
            min_gas,
            GasWeight::default(),
        )
    }

    fn nft_transfer_call(
        self,
        receiver_id: &AccountId,
        token_id: &non_fungible_token::TokenId,
        memo: Option<&str>,
        msg: &str,
        min_gas: Gas,
    ) -> Self {
        self.function_call_weight(
            "nft_transfer_call".to_string(),
            serde_json::to_vec(&json!({
                "receiver_id": receiver_id,
                "token_id": token_id,
                "memo": memo,
                "msg": msg,
            }))
            .unwrap_or_panic_display(),
            NearToken::from_yoctonear(1),
            min_gas,
            GasWeight::default(),
        )
    }
}

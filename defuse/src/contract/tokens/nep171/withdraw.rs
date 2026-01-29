use crate::{
    contract::{Contract, ContractExt, Role, tokens::STORAGE_DEPOSIT_GAS},
    tokens::nep171::{
        NonFungibleTokenForceWithdrawer, NonFungibleTokenWithdrawResolver,
        NonFungibleTokenWithdrawer,
    },
};
use defuse_core::{
    DefuseError, Result,
    engine::StateView,
    intents::tokens::NftWithdraw,
    token_id::{nep141::Nep141TokenId, nep171::Nep171TokenId},
};
use defuse_near_utils::{REFUND_MEMO, UnwrapOrPanic};
use defuse_wnear::{NEAR_WITHDRAW_GAS, ext_wnear};
use near_contract_standards::{
    non_fungible_token::{self, core::ext_nft_core},
    storage_management::ext_storage_management,
};
use near_plugins::{AccessControllable, Pausable, access_control_any, pause};
use near_sdk::{
    AccountId, Gas, NearToken, Promise, PromiseOrValue, assert_one_yocto, env, json_types::U128,
    near, require, serde_json,
};
use std::iter;

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
            self.ensure_auth_predecessor_id(),
            NftWithdraw {
                token,
                receiver_id,
                token_id,
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
    pub(crate) fn internal_nft_withdraw(
        &mut self,
        owner_id: AccountId,
        withdraw: NftWithdraw,
        force: bool,
    ) -> Result<PromiseOrValue<bool>> {
        self.withdraw(
            &owner_id,
            iter::once((
                Nep171TokenId::new(withdraw.token.clone(), withdraw.token_id.clone()).into(),
                1,
            ))
            .chain(withdraw.storage_deposit.map(|amount| {
                (
                    Nep141TokenId::new(self.wnear_id().into_owned()).into(),
                    amount.as_yoctonear(),
                )
            })),
            Some("withdraw"),
            force,
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
            Self::ext(env::current_account_id())
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
    const DO_NFT_WITHDRAW_GAS: Gas = Gas::from_tgas(5)
        // do_nft_withdraw() method is called externally
        // only with storage_deposit
        .saturating_add(STORAGE_DEPOSIT_GAS);

    #[private]
    pub fn do_nft_withdraw(withdraw: NftWithdraw) -> Promise {
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

        let p = ext_nft_core::ext_on(p)
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(min_gas)
            // distribute remaining gas here
            .with_unused_gas_weight(1);
        if let Some(msg) = withdraw.msg {
            p.nft_transfer_call(
                withdraw.receiver_id,
                withdraw.token_id,
                None,
                withdraw.memo,
                msg,
            )
        } else {
            p.nft_transfer(withdraw.receiver_id, withdraw.token_id, None, withdraw.memo)
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
        const MAX_RESULT_LENGTH: usize = "false".len(); // `true` is shorter
        let used = env::promise_result_checked(0, MAX_RESULT_LENGTH).map_or(
            // do not refund on failed `nft_transfer_call` due to
            // NEP-141 vulnerability: `nft_resolve_transfer` fails to
            // read result of `nft_on_transfer` due to insufficient gas
            is_call,
            |value| {
                if is_call {
                    // `nft_transfer_call` returns true if token was successfully transferred
                    serde_json::from_slice::<bool>(&value).unwrap_or_default()
                } else {
                    // `nft_transfer` returns empty result on success
                    value.is_empty()
                }
            },
        );

        if !used {
            self.deposit(
                sender_id,
                [(Nep171TokenId::new(token, token_id).into(), 1)],
                Some(REFUND_MEMO),
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
            true,
        )
        .unwrap_or_panic()
    }
}

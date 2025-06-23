use defuse_core::{
    DefuseError, Nonce, Result,
    crypto::PublicKey,
    engine::{State, StateView},
    fees::Pips,
    intents::tokens::{FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw, StorageDeposit},
    token_id::{TokenId, nep141::Nep141TokenId},
};
use defuse_near_utils::{CURRENT_ACCOUNT_ID, Lock};
use defuse_wnear::{NEAR_WITHDRAW_GAS, ext_wnear};
use near_sdk::{AccountId, AccountIdRef, NearToken, json_types::U128};
use std::borrow::Cow;

use crate::contract::{Contract, accounts::Account};

impl StateView for Contract {
    #[inline]
    fn verifying_contract(&self) -> Cow<'_, AccountIdRef> {
        Cow::Borrowed(CURRENT_ACCOUNT_ID.as_ref())
    }

    #[inline]
    fn wnear_id(&self) -> Cow<'_, AccountIdRef> {
        Cow::Borrowed(self.state.wnear_id.as_ref())
    }

    #[inline]
    fn fee(&self) -> Pips {
        self.state.fees.fee
    }

    #[inline]
    fn fee_collector(&self) -> Cow<'_, AccountIdRef> {
        Cow::Borrowed(self.state.fees.fee_collector.as_ref())
    }

    #[inline]
    fn has_public_key(&self, account_id: &AccountIdRef, public_key: &PublicKey) -> bool {
        self.accounts
            .get(account_id)
            .map(Lock::as_inner_unchecked)
            .map_or_else(
                || account_id == public_key.to_implicit_account_id(),
                |account| account.has_public_key(account_id, public_key),
            )
    }

    fn iter_public_keys(&self, account_id: &AccountIdRef) -> impl Iterator<Item = PublicKey> + '_ {
        let account = self.accounts.get(account_id).map(Lock::as_inner_unchecked);
        account
            .map(|account| account.iter_public_keys(account_id))
            .into_iter()
            .flatten()
            .chain(if account.is_none() {
                PublicKey::from_implicit_account_id(account_id)
            } else {
                None
            })
    }

    #[inline]
    fn is_nonce_used(&self, account_id: &AccountIdRef, nonce: Nonce) -> bool {
        self.accounts
            .get(account_id)
            .map(Lock::as_inner_unchecked)
            .is_some_and(|account| account.is_nonce_used(nonce))
    }

    #[inline]
    fn balance_of(&self, account_id: &AccountIdRef, token_id: &TokenId) -> u128 {
        self.accounts
            .get(account_id)
            .map(Lock::as_inner_unchecked)
            .map(|account| account.token_balances.amount_for(token_id))
            .unwrap_or_default()
    }

    #[inline]
    fn is_account_locked(&self, account_id: &AccountIdRef) -> bool {
        self.accounts.get(account_id).is_some_and(Lock::is_locked)
    }

    #[inline]
    fn is_auth_by_predecessor_id_enabled(&self, account_id: &AccountIdRef) -> bool {
        self.accounts
            .get(account_id)
            .map(Lock::as_inner_unchecked)
            .is_none_or(Account::is_auth_by_predecessor_id_enabled)
    }
}

impl State for Contract {
    #[inline]
    fn add_public_key(&mut self, account_id: AccountId, public_key: PublicKey) -> Result<()> {
        self.accounts
            .get_or_create(account_id.clone())
            .get_mut()
            .ok_or_else(|| DefuseError::AccountLocked(account_id.clone()))?
            .add_public_key(&account_id, public_key)
            .then_some(())
            .ok_or(DefuseError::PublicKeyExists(account_id, public_key))
    }

    #[inline]
    fn remove_public_key(&mut self, account_id: AccountId, public_key: PublicKey) -> Result<()> {
        self.accounts
            .get_or_create(account_id.clone())
            .get_mut()
            .ok_or_else(|| DefuseError::AccountLocked(account_id.clone()))?
            .remove_public_key(&account_id, &public_key)
            .then_some(())
            .ok_or(DefuseError::PublicKeyNotExist(account_id, public_key))
    }

    #[inline]
    fn commit_nonce(&mut self, account_id: AccountId, nonce: Nonce) -> Result<()> {
        self.accounts
            .get_or_create(account_id.clone())
            .get_mut()
            .ok_or(DefuseError::AccountLocked(account_id))?
            .commit_nonce(nonce)
            .then_some(())
            .ok_or(DefuseError::NonceUsed)
    }

    fn internal_add_balance(
        &mut self,
        owner_id: AccountId,
        tokens: impl IntoIterator<Item = (TokenId, u128)>,
    ) -> Result<()> {
        let owner = self
            .accounts
            .get_or_create(owner_id)
            // we allow locked accounts to accept deposits and incoming deposits
            .as_inner_unchecked_mut();

        for (token_id, amount) in tokens {
            if amount == 0 {
                return Err(DefuseError::InvalidIntent);
            }
            owner
                .token_balances
                .add(token_id, amount)
                .ok_or(DefuseError::BalanceOverflow)?;
        }

        Ok(())
    }

    fn internal_sub_balance(
        &mut self,
        owner_id: &AccountIdRef,
        tokens: impl IntoIterator<Item = (TokenId, u128)>,
    ) -> Result<()> {
        let owner = self
            .accounts
            .get_mut(owner_id)
            .ok_or_else(|| DefuseError::AccountNotFound(owner_id.to_owned()))?
            .get_mut()
            .ok_or_else(|| DefuseError::AccountLocked(owner_id.to_owned()))?;

        for (token_id, amount) in tokens {
            if amount == 0 {
                return Err(DefuseError::InvalidIntent);
            }

            owner
                .token_balances
                .sub(token_id.clone(), amount)
                .ok_or(DefuseError::BalanceOverflow)?;
        }

        Ok(())
    }

    fn ft_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: FtWithdraw) -> Result<()> {
        self.internal_ft_withdraw(owner_id.to_owned(), withdraw, false)
            // detach promise
            .map(|_promise| ())
    }

    fn nft_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: NftWithdraw) -> Result<()> {
        self.internal_nft_withdraw(owner_id.to_owned(), withdraw, false)
            // detach promise
            .map(|_promise| ())
    }

    fn mt_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: MtWithdraw) -> Result<()> {
        self.internal_mt_withdraw(owner_id.to_owned(), withdraw, false)
            // detach promise
            .map(|_promise| ())
    }

    fn native_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: NativeWithdraw) -> Result<()> {
        self.withdraw(
            owner_id,
            [(
                Nep141TokenId::new(self.wnear_id().into_owned()).into(),
                withdraw.amount.as_yoctonear(),
            )],
            Some("withdraw"),
            false,
        )?;

        // detach promise
        let _ = ext_wnear::ext(self.wnear_id.clone())
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(NEAR_WITHDRAW_GAS)
            // do not distribute remaining gas here
            .with_unused_gas_weight(0)
            .near_withdraw(U128(withdraw.amount.as_yoctonear()))
            .then(
                // do_native_withdraw only after unwrapping NEAR
                Self::ext(CURRENT_ACCOUNT_ID.clone())
                    .with_static_gas(Self::DO_NATIVE_WITHDRAW_GAS)
                    // do not distribute remaining gas here
                    .with_unused_gas_weight(0)
                    .do_native_withdraw(withdraw),
            );

        Ok(())
    }

    fn storage_deposit(
        &mut self,
        owner_id: &AccountIdRef,
        storage_deposit: StorageDeposit,
    ) -> Result<()> {
        self.withdraw(
            owner_id,
            [(
                Nep141TokenId::new(self.wnear_id().into_owned()).into(),
                storage_deposit.amount.as_yoctonear(),
            )],
            Some("withdraw"),
            false,
        )?;

        // detach promise
        let _ = ext_wnear::ext(self.wnear_id.clone())
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(NEAR_WITHDRAW_GAS)
            // do not distribute remaining gas here
            .with_unused_gas_weight(0)
            .near_withdraw(U128(storage_deposit.amount.as_yoctonear()))
            .then(
                // do_storage_deposit only after unwrapping NEAR
                Self::ext(CURRENT_ACCOUNT_ID.clone())
                    .with_static_gas(Self::DO_STORAGE_DEPOSIT_GAS)
                    // do not distribute remaining gas here
                    .with_unused_gas_weight(0)
                    .do_storage_deposit(storage_deposit),
            );

        Ok(())
    }

    fn set_auth_by_predecessor_id(&mut self, account_id: AccountId, enable: bool) -> Result<bool> {
        if enable {
            let Some(account) = self.accounts.get_mut(&account_id) else {
                // no need to create an account: not-yet-existing accounts
                // have auth by PREDECESSOR_ID enabled by default
                return Ok(true);
            };
            account
        } else {
            self.accounts.get_or_create(account_id.clone())
        }
        .get_mut()
        .ok_or_else(|| DefuseError::AccountLocked(account_id.clone()))
        .map(|account| account.set_auth_by_predecessor_id(&account_id, enable))
    }
}

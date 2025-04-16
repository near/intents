use std::borrow::Cow;

use defuse_core::{
    DefuseError, Nonce, Result,
    crypto::PublicKey,
    engine::{State, StateView},
    fees::Pips,
    intents::tokens::{FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw, StorageDeposit},
    tokens::TokenId,
};
use defuse_near_utils::{CURRENT_ACCOUNT_ID, Lock};
use defuse_wnear::{NEAR_WITHDRAW_GAS, ext_wnear};
use near_sdk::{AccountId, AccountIdRef, NearToken, json_types::U128};

use crate::contract::Contract;

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
}

impl State for Contract {
    #[inline]
    fn add_public_key(&mut self, account_id: AccountId, public_key: PublicKey) -> Result<()> {
        self.accounts
            .get_or_create(account_id.clone())
            .as_unlocked_mut()
            // TODO: allow changing locked account state by permissioned accounts
            .ok_or(DefuseError::AccountLocked)?
            .add_public_key(&account_id, public_key)
            .then_some(())
            .ok_or(DefuseError::PublicKeyExists)
    }

    #[inline]
    fn remove_public_key(&mut self, account_id: AccountId, public_key: PublicKey) -> Result<()> {
        self.accounts
            // create account if doesn't exist, so the user can opt out of implicit public key
            .get_or_create(account_id.clone())
            .as_unlocked_mut()
            // TODO: allow changing locked account state by permissioned accounts
            .ok_or(DefuseError::AccountLocked)?
            .remove_public_key(&account_id, &public_key)
            .then_some(())
            .ok_or(DefuseError::PublicKeyNotExist)
    }

    #[inline]
    fn commit_nonce(&mut self, account_id: AccountId, nonce: Nonce) -> Result<()> {
        self.accounts
            .get_or_create(account_id)
            .as_unlocked_mut()
            // TODO: allow changing locked account state by permissioned accounts
            .ok_or(DefuseError::AccountLocked)?
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
            .as_unlocked_mut()
            // TODO: allow changing locked account state by permissioned accounts
            .ok_or(DefuseError::AccountLocked)?;

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
            .ok_or(DefuseError::AccountNotFound)?
            .as_unlocked_mut()
            // TODO: allow changing locked account state by permissioned accounts
            .ok_or(DefuseError::AccountLocked)?;

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
                TokenId::Nep141(self.wnear_id().into_owned()),
                withdraw.amount.as_yoctonear(),
            )],
            Some("withdraw"),
            false,
        )?;

        // detach promise
        let _ = ext_wnear::ext(self.wnear_id.clone())
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(NEAR_WITHDRAW_GAS)
            .near_withdraw(U128(withdraw.amount.as_yoctonear()))
            .then(
                // do_native_withdraw only after unwrapping NEAR
                Contract::ext(CURRENT_ACCOUNT_ID.clone())
                    .with_static_gas(Contract::DO_NATIVE_WITHDRAW_GAS)
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
                TokenId::Nep141(self.wnear_id().into_owned()),
                storage_deposit.amount.as_yoctonear(),
            )],
            Some("withdraw"),
            false,
        )?;

        // detach promise
        let _ = ext_wnear::ext(self.wnear_id.clone())
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(NEAR_WITHDRAW_GAS)
            .near_withdraw(U128(storage_deposit.amount.as_yoctonear()))
            .then(
                // do_storage_deposit only after unwrapping NEAR
                Contract::ext(CURRENT_ACCOUNT_ID.clone())
                    .with_static_gas(Contract::DO_STORAGE_DEPOSIT_GAS)
                    .do_storage_deposit(storage_deposit),
            );

        Ok(())
    }
}

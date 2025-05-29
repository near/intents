use defuse_core::{
    DefuseError, Nonce, Result,
    crypto::PublicKey,
    engine::{State, StateView},
    fees::Pips,
    intents::tokens::{FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw, StorageDeposit},
    tokens::TokenId,
};
use defuse_near_utils::CURRENT_ACCOUNT_ID;
use defuse_wnear::{NEAR_WITHDRAW_GAS, ext_wnear};
use near_sdk::{AccountId, AccountIdRef, NearToken, json_types::U128, require};
use std::borrow::Cow;

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
        self.accounts.get(account_id).map_or_else(
            || account_id == public_key.to_implicit_account_id(),
            |account| account.has_public_key(account_id, public_key),
        )
    }

    fn iter_public_keys(&self, account_id: &AccountIdRef) -> impl Iterator<Item = PublicKey> + '_ {
        let account = self.accounts.get(account_id);
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
            .is_some_and(|account| account.is_nonce_used(nonce))
    }

    #[inline]
    fn balance_of(&self, account_id: &AccountIdRef, token_id: &TokenId) -> u128 {
        self.accounts
            .get(account_id)
            .map(|account| account.token_balances.amount_for(token_id))
            .unwrap_or_default()
    }
}

impl State for Contract {
    #[inline]
    fn add_public_key(&mut self, account_id: AccountId, public_key: PublicKey) -> bool {
        self.accounts
            .get_or_create(account_id.clone())
            .add_public_key(&account_id, public_key)
    }

    #[inline]
    fn remove_public_key(&mut self, account_id: AccountId, public_key: PublicKey) -> bool {
        self.accounts
            .get_or_create(account_id.clone())
            .remove_public_key(&account_id, &public_key)
    }

    #[inline]
    fn commit_nonce(&mut self, account_id: AccountId, nonce: Nonce) -> bool {
        self.accounts.get_or_create(account_id).commit_nonce(nonce)
    }

    fn internal_add_balance(
        &mut self,
        owner_id: AccountId,
        tokens: impl IntoIterator<Item = (TokenId, u128)>,
    ) -> Result<()> {
        let owner = self.accounts.get_or_create(owner_id);

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
            .ok_or(DefuseError::AccountNotFound)?;

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
        self.internal_ft_withdraw(owner_id.to_owned(), withdraw)
            // detach promise
            .map(|_promise| ())
    }

    fn nft_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: NftWithdraw) -> Result<()> {
        self.internal_nft_withdraw(owner_id.to_owned(), withdraw)
            // detach promise
            .map(|_promise| ())
    }

    fn mt_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: MtWithdraw) -> Result<()> {
        self.internal_mt_withdraw(owner_id.to_owned(), withdraw)
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
        )?;

        // detach promise
        let _ = ext_wnear::ext(self.wnear_id.clone())
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(NEAR_WITHDRAW_GAS)
            .near_withdraw(U128(withdraw.amount.as_yoctonear()))
            .then(
                // do_native_withdraw only after unwrapping NEAR
                Self::ext(CURRENT_ACCOUNT_ID.clone())
                    .with_static_gas(Self::DO_NATIVE_WITHDRAW_GAS)
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
        )?;

        // detach promise
        let _ = ext_wnear::ext(self.wnear_id.clone())
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(NEAR_WITHDRAW_GAS)
            .near_withdraw(U128(storage_deposit.amount.as_yoctonear()))
            .then(
                // do_storage_deposit only after unwrapping NEAR
                Self::ext(CURRENT_ACCOUNT_ID.clone())
                    .with_static_gas(Self::DO_STORAGE_DEPOSIT_GAS)
                    .do_storage_deposit(storage_deposit),
            );

        Ok(())
    }

    fn set_fee(&mut self, mut fee: Pips) {
        require!(self.fees.fee != fee, "same");
        std::mem::swap(&mut self.fees.fee, &mut fee);
    }

    fn set_fee_collector(&mut self, mut fee_collector: AccountId) {
        require!(self.fees.fee_collector != fee_collector, "same");
        std::mem::swap(&mut self.fees.fee_collector, &mut fee_collector);
    }

    fn internal_mt_batch_transfer(
        &mut self,
        sender_id: &AccountIdRef,
        receiver_id: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<near_sdk::json_types::U128>,
        _memo: Option<&str>,
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

        Ok(())
    }
}

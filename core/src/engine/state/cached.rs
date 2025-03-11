use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use defuse_bitmap::{U248, U256};
use defuse_crypto::PublicKey;
use near_sdk::{AccountId, AccountIdRef};

use crate::{
    DefuseError, Nonce, Nonces, Result,
    fees::Pips,
    intents::tokens::{FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw},
    tokens::{Amounts, TokenId},
};

use super::{State, StateView};

#[derive(Debug)]
pub struct CachedState<W: StateView> {
    view: W,
    accounts: CachedAccounts,
}

impl<W> CachedState<W>
where
    W: StateView,
{
    #[inline]
    pub fn new(view: W) -> Self {
        Self {
            view,
            accounts: CachedAccounts::new(),
        }
    }
}

impl<W> StateView for CachedState<W>
where
    W: StateView,
{
    #[inline]
    fn verifying_contract(&self) -> Cow<'_, AccountIdRef> {
        self.view.verifying_contract()
    }

    #[inline]
    fn wnear_id(&self) -> Cow<'_, AccountIdRef> {
        self.view.wnear_id()
    }

    #[inline]
    fn fee(&self) -> Pips {
        self.view.fee()
    }

    #[inline]
    fn fee_collector(&self) -> Cow<'_, AccountIdRef> {
        self.view.fee_collector()
    }

    fn has_public_key(&self, account_id: &AccountIdRef, public_key: &PublicKey) -> bool {
        if let Some(account) = self.accounts.get(account_id) {
            if account.public_keys_added.contains(public_key) {
                return true;
            }
            if account.public_keys_removed.contains(public_key) {
                return false;
            }
        }
        self.view.has_public_key(account_id, public_key)
    }

    fn iter_public_keys(&self, account_id: &AccountIdRef) -> impl Iterator<Item = PublicKey> + '_ {
        let account = self.accounts.get(account_id);
        self.view
            .iter_public_keys(account_id)
            .filter(move |pk| account.is_none_or(|a| !a.public_keys_removed.contains(pk)))
            .chain(
                account
                    .map(|a| &a.public_keys_added)
                    .into_iter()
                    .flatten()
                    .copied(),
            )
    }

    fn is_nonce_used(&self, account_id: &AccountIdRef, nonce: Nonce) -> bool {
        self.accounts
            .get(account_id)
            .is_some_and(|account| account.is_nonce_used(nonce))
            || self.view.is_nonce_used(account_id, nonce)
    }

    fn balance_of(&self, account_id: &AccountIdRef, token_id: &TokenId) -> u128 {
        self.accounts
            .get(account_id)
            .and_then(|account| account.token_amounts.get(token_id).copied())
            .unwrap_or_else(|| self.view.balance_of(account_id, token_id))
    }
}

impl<W> State for CachedState<W>
where
    W: StateView,
{
    #[must_use]
    fn add_public_key(&mut self, account_id: AccountId, public_key: PublicKey) -> bool {
        let had = self.has_public_key(&account_id, &public_key);
        let account = self.accounts.get_or_create(account_id.clone());
        if had {
            account.public_keys_removed.remove(&public_key)
        } else {
            account.public_keys_added.insert(public_key)
        }
    }

    #[must_use]
    fn remove_public_key(&mut self, account_id: AccountId, public_key: PublicKey) -> bool {
        let had = self.has_public_key(&account_id, &public_key);
        let account = self.accounts.get_or_create(account_id.clone());
        if had {
            account.public_keys_removed.insert(public_key)
        } else {
            account.public_keys_added.remove(&public_key)
        }
    }

    #[must_use]
    fn commit_nonce(&mut self, account_id: AccountId, nonce: Nonce) -> bool {
        if self.is_nonce_used(&account_id, nonce) {
            return false;
        }
        self.accounts.get_or_create(account_id).commit_nonce(nonce)
    }

    fn internal_add_balance(
        &mut self,
        owner_id: AccountId,
        token_amounts: impl IntoIterator<Item = (TokenId, u128)>,
    ) -> Result<()> {
        let account = self.accounts.get_or_create(owner_id.clone());
        for (token_id, amount) in token_amounts {
            if account.token_amounts.get(&token_id).is_none() {
                account
                    .token_amounts
                    .add(token_id.clone(), self.view.balance_of(&owner_id, &token_id))
                    .ok_or(DefuseError::BalanceOverflow)?;
            }
            account
                .token_amounts
                .add(token_id, amount)
                .ok_or(DefuseError::BalanceOverflow)?;
        }
        Ok(())
    }

    fn internal_sub_balance(
        &mut self,
        owner_id: &AccountIdRef,
        token_amounts: impl IntoIterator<Item = (TokenId, u128)>,
    ) -> Result<()> {
        let account = self
            .accounts
            .get_mut(owner_id)
            .ok_or(DefuseError::AccountNotFound)?;
        for (token_id, amount) in token_amounts {
            if amount == 0 {
                return Err(DefuseError::InvalidIntent);
            }

            if account.token_amounts.get(&token_id).is_none() {
                account
                    .token_amounts
                    .add(token_id.clone(), self.view.balance_of(owner_id, &token_id))
                    .ok_or(DefuseError::BalanceOverflow)?;
            }
            account
                .token_amounts
                .sub(token_id, amount)
                .ok_or(DefuseError::BalanceOverflow)?;
        }
        Ok(())
    }

    fn ft_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: FtWithdraw) -> Result<()> {
        self.internal_sub_balance(
            owner_id,
            std::iter::once((TokenId::Nep141(withdraw.token.clone()), withdraw.amount.0)).chain(
                withdraw.storage_deposit.map(|amount| {
                    (
                        TokenId::Nep141(self.wnear_id().into_owned()),
                        amount.as_yoctonear(),
                    )
                }),
            ),
        )
    }

    fn nft_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: NftWithdraw) -> Result<()> {
        self.internal_sub_balance(
            owner_id,
            std::iter::once((
                TokenId::Nep171(withdraw.token.clone(), withdraw.token_id.clone()),
                1,
            ))
            .chain(withdraw.storage_deposit.map(|amount| {
                (
                    TokenId::Nep141(self.wnear_id().into_owned()),
                    amount.as_yoctonear(),
                )
            })),
        )
    }

    fn mt_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: MtWithdraw) -> Result<()> {
        if withdraw.token_ids.len() != withdraw.amounts.len() || withdraw.token_ids.is_empty() {
            return Err(DefuseError::InvalidIntent);
        }

        self.internal_sub_balance(
            owner_id,
            std::iter::repeat(withdraw.token.clone())
                .zip(withdraw.token_ids.iter().cloned())
                .map(|(token, token_id)| TokenId::Nep245(token, token_id))
                .zip(withdraw.amounts.iter().map(|a| a.0))
                .chain(withdraw.storage_deposit.map(|amount| {
                    (
                        TokenId::Nep141(self.wnear_id().into_owned()),
                        amount.as_yoctonear(),
                    )
                })),
        )
    }

    fn native_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: NativeWithdraw) -> Result<()> {
        self.internal_sub_balance(
            owner_id,
            [(
                TokenId::Nep141(self.wnear_id().into_owned()),
                withdraw.amount.as_yoctonear(),
            )],
        )
    }
}

#[derive(Debug, Default, Clone)]
pub struct CachedAccounts(HashMap<AccountId, CachedAccount>);

impl CachedAccounts {
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    #[inline]
    pub fn get(&self, account_id: &AccountIdRef) -> Option<&CachedAccount> {
        self.0.get(account_id)
    }

    #[inline]
    pub fn get_mut(&mut self, account_id: &AccountIdRef) -> Option<&mut CachedAccount> {
        self.0.get_mut(account_id)
    }

    #[inline]
    pub fn get_or_create(&mut self, account_id: AccountId) -> &mut CachedAccount {
        self.0.entry(account_id).or_default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct CachedAccount {
    nonces: Nonces<HashMap<U248, U256>>,

    public_keys_added: HashSet<PublicKey>,
    public_keys_removed: HashSet<PublicKey>,

    token_amounts: Amounts<HashMap<TokenId, u128>>,
}

impl CachedAccount {
    #[inline]
    pub fn is_nonce_used(&self, nonce: U256) -> bool {
        self.nonces.is_used(nonce)
    }

    #[inline]
    pub fn commit_nonce(&mut self, n: U256) -> bool {
        self.nonces.commit(n)
    }
}

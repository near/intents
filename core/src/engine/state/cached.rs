use crate::{
    DefuseError, Nonce, Nonces, Result,
    amounts::Amounts,
    fees::Pips,
    intents::{
        auth::AuthCall,
        tokens::{FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw, StorageDeposit},
    },
    token_id::{TokenId, nep141::Nep141TokenId, nep171::Nep171TokenId, nep245::Nep245TokenId},
};
use defuse_bitmap::{U248, U256};
use defuse_crypto::PublicKey;
use defuse_near_utils::Lock;
use near_sdk::{AccountId, AccountIdRef};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
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
        if let Some(account) = self.accounts.get(account_id).map(Lock::as_inner_unchecked) {
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
        let account = self.accounts.get(account_id).map(Lock::as_inner_unchecked);
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
            .map(Lock::as_inner_unchecked)
            .is_some_and(|account| account.is_nonce_used(nonce))
            || self.view.is_nonce_used(account_id, nonce)
    }

    fn balance_of(&self, account_id: &AccountIdRef, token_id: &TokenId) -> u128 {
        self.accounts
            .get(account_id)
            .map(Lock::as_inner_unchecked)
            .and_then(|account| account.token_amounts.get(token_id).copied())
            .unwrap_or_else(|| self.view.balance_of(account_id, token_id))
    }

    fn is_account_locked(&self, account_id: &AccountIdRef) -> bool {
        self.accounts
            .get(account_id)
            .map_or_else(|| self.view.is_account_locked(account_id), Lock::is_locked)
    }

    fn is_auth_by_predecessor_id_enabled(&self, account_id: &AccountIdRef) -> bool {
        let was_enabled = self.view.is_auth_by_predecessor_id_enabled(account_id);
        let toggled = self
            .accounts
            .get(account_id)
            .map(Lock::as_inner_unchecked)
            .is_some_and(|a| a.auth_by_predecessor_id_toggled);
        was_enabled ^ toggled
    }
}

impl<W> State for CachedState<W>
where
    W: StateView,
{
    fn add_public_key(&mut self, account_id: AccountId, public_key: PublicKey) -> Result<()> {
        let had = self.view.has_public_key(&account_id, &public_key);
        let account = self
            .accounts
            .get_or_create(account_id.clone(), |account_id| {
                self.view.is_account_locked(account_id)
            })
            .get_mut()
            .ok_or_else(|| DefuseError::AccountLocked(account_id.clone()))?;
        let added = if had {
            account.public_keys_removed.remove(&public_key)
        } else {
            account.public_keys_added.insert(public_key)
        };
        if !added {
            return Err(DefuseError::PublicKeyExists(account_id, public_key));
        }
        Ok(())
    }

    fn remove_public_key(&mut self, account_id: AccountId, public_key: PublicKey) -> Result<()> {
        let had = self.view.has_public_key(&account_id, &public_key);
        let account = self
            .accounts
            .get_or_create(account_id.clone(), |account_id| {
                self.view.is_account_locked(account_id)
            })
            .get_mut()
            .ok_or_else(|| DefuseError::AccountLocked(account_id.clone()))?;
        let removed = if had {
            account.public_keys_removed.insert(public_key)
        } else {
            account.public_keys_added.remove(&public_key)
        };
        if !removed {
            return Err(DefuseError::PublicKeyNotExist(account_id, public_key));
        }
        Ok(())
    }

    fn commit_nonce(&mut self, account_id: AccountId, nonce: Nonce) -> Result<()> {
        self.accounts
            .get_or_create(account_id.clone(), |account_id| {
                self.view.is_account_locked(account_id)
            })
            .get_mut()
            .ok_or(DefuseError::AccountLocked(account_id))?
            .commit_nonce(nonce)
    }

    fn cleanup_expired_nonces(
        &mut self,
        account_id: &AccountId,
        nonces: impl IntoIterator<Item = Nonce>,
    ) -> Result<()> {
        let account = self
            .accounts
            .get_mut(account_id)
            .ok_or_else(|| DefuseError::AccountNotFound(account_id.clone()))?
            .as_inner_unchecked_mut();

        for n in nonces {
            account.clear_expired_nonce(n);
        }

        Ok(())
    }

    fn internal_add_balance(
        &mut self,
        owner_id: AccountId,
        token_amounts: impl IntoIterator<Item = (TokenId, u128)>,
    ) -> Result<()> {
        let account = self
            .accounts
            .get_or_create(owner_id.clone(), |owner_id| {
                self.view.is_account_locked(owner_id)
            })
            .as_inner_unchecked_mut();
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
            .get_or_create(owner_id.to_owned(), |owner_id| {
                self.view.is_account_locked(owner_id)
            })
            .get_mut()
            .ok_or_else(|| DefuseError::AccountLocked(owner_id.to_owned()))?;
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
            std::iter::once((
                Nep141TokenId::new(withdraw.token.clone()).into(),
                withdraw.amount.0,
            ))
            .chain(withdraw.storage_deposit.map(|amount| {
                (
                    Nep141TokenId::new(self.wnear_id().into_owned()).into(),
                    amount.as_yoctonear(),
                )
            })),
        )
    }

    fn nft_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: NftWithdraw) -> Result<()> {
        self.internal_sub_balance(
            owner_id,
            std::iter::once((
                Nep171TokenId::new(withdraw.token.clone(), withdraw.token_id.clone())?.into(),
                1,
            ))
            .chain(withdraw.storage_deposit.map(|amount| {
                (
                    Nep141TokenId::new(self.wnear_id().into_owned()).into(),
                    amount.as_yoctonear(),
                )
            })),
        )
    }

    fn mt_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: MtWithdraw) -> Result<()> {
        if withdraw.token_ids.len() != withdraw.amounts.len() || withdraw.token_ids.is_empty() {
            return Err(DefuseError::InvalidIntent);
        }

        let token_ids = std::iter::repeat(withdraw.token.clone())
            .zip(withdraw.token_ids.iter().cloned())
            .map(|(token, token_id)| Nep245TokenId::new(token, token_id))
            .collect::<Result<Vec<_>, _>>()?;

        self.internal_sub_balance(
            owner_id,
            token_ids
                .into_iter()
                .map(Into::into)
                .zip(withdraw.amounts.iter().map(|a| a.0))
                .chain(
                    withdraw
                        .storage_deposit
                        .map(|amount| (self.wnear_token_id(), amount.as_yoctonear())),
                ),
        )
    }

    fn native_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: NativeWithdraw) -> Result<()> {
        self.internal_sub_balance(
            owner_id,
            [(
                Nep141TokenId::new(self.wnear_id().into_owned()).into(),
                withdraw.amount.as_yoctonear(),
            )],
        )
    }

    fn storage_deposit(
        &mut self,
        owner_id: &AccountIdRef,
        storage_deposit: StorageDeposit,
    ) -> Result<()> {
        self.internal_sub_balance(
            owner_id,
            [(
                Nep141TokenId::new(self.wnear_id().into_owned()).into(),
                storage_deposit.amount.as_yoctonear(),
            )],
        )
    }

    fn set_auth_by_predecessor_id(&mut self, account_id: AccountId, enable: bool) -> Result<bool> {
        let was_enabled = self.is_auth_by_predecessor_id_enabled(&account_id);
        let toggle = was_enabled ^ enable;
        if toggle {
            self.accounts
                .get_or_create(account_id.clone(), |owner_id| {
                    self.view.is_account_locked(owner_id)
                })
                .get_mut()
                .ok_or(DefuseError::AccountLocked(account_id))?
                // toggle
                .auth_by_predecessor_id_toggled ^= true;
        }
        Ok(was_enabled)
    }

    fn auth_call(&mut self, signer_id: &AccountIdRef, auth_call: AuthCall) -> Result<()> {
        if !auth_call.attached_deposit.is_zero() {
            self.internal_sub_balance(
                signer_id,
                [(
                    Nep141TokenId::new(self.wnear_id().into_owned()).into(),
                    auth_call.attached_deposit.as_yoctonear(),
                )],
            )?;
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct CachedAccounts(HashMap<AccountId, Lock<CachedAccount>>);

impl CachedAccounts {
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    #[inline]
    pub fn get(&self, account_id: &AccountIdRef) -> Option<&Lock<CachedAccount>> {
        self.0.get(account_id)
    }

    #[inline]
    pub fn get_mut(&mut self, account_id: &AccountIdRef) -> Option<&mut Lock<CachedAccount>> {
        self.0.get_mut(account_id)
    }

    #[inline]
    pub fn get_or_create(
        &mut self,
        account_id: AccountId,
        is_initially_locked: impl FnOnce(&AccountId) -> bool,
    ) -> &mut Lock<CachedAccount> {
        self.0.entry(account_id).or_insert_with_key(|account_id| {
            Lock::new(is_initially_locked(account_id), CachedAccount::default())
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct CachedAccount {
    nonces: Nonces<HashMap<U248, U256>>,

    auth_by_predecessor_id_toggled: bool,

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
    pub fn commit_nonce(&mut self, n: U256) -> Result<()> {
        self.nonces.commit(n)
    }

    #[inline]
    pub fn clear_expired_nonce(&mut self, n: U256) -> bool {
        self.nonces.clear_expired(n)
    }
}

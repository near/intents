mod account;
mod state;

pub use self::{account::*, state::*};

use std::collections::HashSet;

use defuse_borsh_utils::r#as::AsWrap;
use defuse_core::{
    Nonce,
    crypto::PublicKey,
    engine::{State, StateView},
};
use defuse_near_utils::{Lock, MaybeLock, NestPrefix, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic};
use defuse_serde_utils::base64::AsBase64;
use near_plugins::{AccessControllable, access_control_any};
use near_sdk::{
    AccountId, AccountIdRef, BorshStorageKey, IntoStorageKey, assert_one_yocto,
    borsh::BorshSerialize, near, store::IterableMap,
};

use crate::{
    accounts::{AccountForceLocker, AccountManager},
    contract::{Contract, ContractExt, Role},
};

#[near]
impl AccountManager for Contract {
    fn has_public_key(&self, account_id: &AccountId, public_key: &PublicKey) -> bool {
        StateView::has_public_key(&self, account_id, public_key)
    }

    fn public_keys_of(&self, account_id: &AccountId) -> HashSet<PublicKey> {
        StateView::iter_public_keys(&self, account_id).collect()
    }

    #[payable]
    fn add_public_key(&mut self, public_key: PublicKey) {
        assert_one_yocto();
        State::add_public_key(self, PREDECESSOR_ACCOUNT_ID.clone(), public_key).unwrap_or_panic();
    }

    #[payable]
    fn remove_public_key(&mut self, public_key: PublicKey) {
        assert_one_yocto();
        State::remove_public_key(self, PREDECESSOR_ACCOUNT_ID.clone(), public_key)
            .unwrap_or_panic();
    }

    fn is_nonce_used(&self, account_id: &AccountId, nonce: AsBase64<Nonce>) -> bool {
        StateView::is_nonce_used(&self, account_id, nonce.into_inner())
    }

    #[payable]
    fn invalidate_nonces(&mut self, nonces: Vec<AsBase64<Nonce>>) {
        assert_one_yocto();
        nonces
            .into_iter()
            .map(AsBase64::into_inner)
            .try_for_each(|n| State::commit_nonce(self, PREDECESSOR_ACCOUNT_ID.clone(), n))
            .unwrap_or_panic();
    }
}

#[derive(Debug)]
#[near(serializers = [borsh])]
pub struct Accounts {
    // Unfortunately, we can't use `#[borsh(deserialize_with = "...")]` here,
    // as IterableMap requires K and V parameters to implement BorshSerialize
    accounts: IterableMap<AccountId, AsWrap<Lock<Account>, MaybeLock>>,
    prefix: Vec<u8>,
}

impl Accounts {
    #[inline]
    pub fn new<S>(prefix: S) -> Self
    where
        S: IntoStorageKey,
    {
        let prefix = prefix.into_storage_key();
        Self {
            accounts: IterableMap::new(prefix.as_slice().nest(AccountsPrefix::Accounts)),
            prefix,
        }
    }

    #[inline]
    pub fn get(&self, account_id: &AccountIdRef) -> Option<&Lock<Account>> {
        self.accounts.get(account_id).map(|a| &**a)
    }

    #[inline]
    pub fn get_mut(&mut self, account_id: &AccountIdRef) -> Option<&mut Lock<Account>> {
        self.accounts.get_mut(account_id).map(|a| &mut **a)
    }

    // TODO: docs
    // Creates unlocked account by default
    #[inline]
    pub fn get_or_create(&mut self, account_id: AccountId) -> &mut Lock<Account> {
        // TODO: allow creating locked accounts
        self.accounts
            .entry(account_id)
            .or_insert_with_key(|account_id| {
                Lock::unlocked(Account::new(
                    self.prefix
                        .as_slice()
                        .nest(AccountsPrefix::Account(account_id)),
                    account_id,
                ))
                .into()
            })
    }
}

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "::near_sdk::borsh")]
enum AccountsPrefix<'a> {
    Accounts,
    Account(&'a AccountId),
}

#[near]
impl AccountForceLocker for Contract {
    fn is_account_locked(&self, account_id: &AccountId) -> bool {
        self.accounts.get(account_id).is_some_and(Lock::is_locked)
    }

    #[access_control_any(roles(Role::DAO, Role::UnrestrictedAccountLocker))]
    #[payable]
    fn force_lock_account(&mut self, account_id: AccountId) -> bool {
        assert_one_yocto();
        self.accounts.get_or_create(account_id).lock().is_some()
    }

    #[access_control_any(roles(Role::DAO, Role::UnrestrictedAccountUnlocker))]
    #[payable]
    fn force_unlock_account(&mut self, account_id: &AccountId) -> bool {
        assert_one_yocto();
        self.accounts
            .get_mut(account_id)
            .and_then(Lock::unlock)
            .is_some()
    }
}

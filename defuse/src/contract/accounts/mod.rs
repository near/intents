mod account;
mod lock;
mod state;

pub use self::{account::*, state::*};

use std::collections::HashSet;

use defuse_core::{
    DefuseError, Nonce,
    crypto::PublicKey,
    engine::{State, StateView},
};
use defuse_near_utils::{Lock, NestPrefix, PREDECESSOR_ACCOUNT_ID, UnwrapOrPanic};
use defuse_serde_utils::base64::AsBase64;

use near_sdk::{
    AccountId, AccountIdRef, BorshStorageKey, FunctionError, IntoStorageKey, assert_one_yocto,
    borsh::BorshSerialize, near, store::IterableMap,
};

use crate::{
    accounts::AccountManager,
    contract::{Contract, ContractExt, accounts::AccountEntry},
};

#[near]
impl AccountManager for Contract {
    fn has_public_key(&self, account_id: &AccountId, public_key: &PublicKey) -> bool {
        StateView::has_public_key(self, account_id, public_key)
    }

    fn public_keys_of(&self, account_id: &AccountId) -> HashSet<PublicKey> {
        StateView::iter_public_keys(self, account_id).collect()
    }

    #[payable]
    fn add_public_key(&mut self, public_key: PublicKey) {
        assert_one_yocto();
        State::add_public_key(self, self.ensure_auth_predecessor_id().clone(), public_key)
            .unwrap_or_panic();
    }

    #[payable]
    fn remove_public_key(&mut self, public_key: PublicKey) {
        assert_one_yocto();
        State::remove_public_key(self, self.ensure_auth_predecessor_id().clone(), public_key)
            .unwrap_or_panic();
    }

    fn is_nonce_used(&self, account_id: &AccountId, nonce: AsBase64<Nonce>) -> bool {
        StateView::is_nonce_used(self, account_id, nonce.into_inner())
    }

    fn is_auth_by_predecessor_id_enabled(&self, account_id: &AccountId) -> bool {
        StateView::is_auth_by_predecessor_id_enabled(self, account_id)
    }

    #[payable]
    fn disable_auth_by_predecessor_id(&mut self) {
        assert_one_yocto();
        State::set_auth_by_predecessor_id(self, self.ensure_auth_predecessor_id().clone(), false)
            .unwrap_or_panic();
    }
}

impl Contract {
    #[inline]
    pub fn ensure_auth_predecessor_id(&self) -> &'static AccountId {
        if !StateView::is_auth_by_predecessor_id_enabled(self, &PREDECESSOR_ACCOUNT_ID) {
            DefuseError::AuthByPredecessorIdDisabled(PREDECESSOR_ACCOUNT_ID.clone()).panic();
        }
        &PREDECESSOR_ACCOUNT_ID
    }
}

#[derive(Debug)]
#[near(serializers = [borsh])]
pub struct Accounts {
    accounts: IterableMap<AccountId, AccountEntry>,
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

    /// Gets or creates an account with given `account_id`.
    /// NOTE: The created account will be unblocked by default.
    #[inline]
    pub fn get_or_create(&mut self, account_id: AccountId) -> &mut Lock<Account> {
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
    Account(&'a AccountIdRef),
}

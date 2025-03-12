mod account;
mod state;

pub use self::{account::*, state::*};

use std::collections::HashSet;

use defuse_core::{DefuseError, Nonce, crypto::PublicKey};
use defuse_near_utils::{NestPrefix, PREDECESSOR_ACCOUNT_ID};
use defuse_serde_utils::base64::AsBase64;
use near_sdk::{
    AccountId, AccountIdRef, BorshStorageKey, FunctionError, IntoStorageKey, assert_one_yocto,
    borsh::BorshSerialize, near, store::IterableMap,
};

use crate::{
    accounts::AccountManager,
    contract::{Contract, ContractExt},
};

#[near]
impl AccountManager for Contract {
    fn has_public_key(&self, account_id: &AccountId, public_key: &PublicKey) -> bool {
        self.accounts.get(account_id).map_or_else(
            || account_id == &public_key.to_implicit_account_id(),
            |account| account.has_public_key(account_id, public_key),
        )
    }

    fn public_keys_of(&self, account_id: &AccountId) -> HashSet<PublicKey> {
        self.accounts.get(account_id).map_or_else(
            || {
                PublicKey::from_implicit_account_id(account_id)
                    .into_iter()
                    .collect()
            },
            |account| account.iter_public_keys(account_id).collect(),
        )
    }

    #[payable]
    fn add_public_key(&mut self, public_key: PublicKey) {
        assert_one_yocto();
        if !self
            .accounts
            .get_or_create(PREDECESSOR_ACCOUNT_ID.clone())
            .add_public_key(&PREDECESSOR_ACCOUNT_ID, public_key)
        {
            DefuseError::PublicKeyExists.panic()
        }
    }

    #[payable]
    fn remove_public_key(&mut self, public_key: &PublicKey) {
        assert_one_yocto();
        if !self
            .accounts
            // create account if doesn't exist, so the user can opt out of implicit public key
            .get_or_create(PREDECESSOR_ACCOUNT_ID.clone())
            .remove_public_key(&PREDECESSOR_ACCOUNT_ID, public_key)
        {
            DefuseError::PublicKeyNotExist.panic()
        }
    }

    fn is_nonce_used(&self, account_id: &AccountId, nonce: AsBase64<Nonce>) -> bool {
        self.accounts
            .get(account_id)
            .is_some_and(move |account| account.is_nonce_used(nonce.into_inner()))
    }

    #[payable]
    fn invalidate_nonces(&mut self, nonces: Vec<AsBase64<Nonce>>) {
        assert_one_yocto();
        let account = self.accounts.get_or_create(PREDECESSOR_ACCOUNT_ID.clone());
        for n in nonces.into_iter().map(AsBase64::into_inner) {
            if !account.commit_nonce(n) {
                DefuseError::NonceUsed.panic()
            }
        }
    }
}

#[derive(Debug)]
#[near(serializers = [borsh])]
pub struct Accounts {
    accounts: IterableMap<AccountId, Account>,
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
    pub fn get(&self, account_id: &AccountIdRef) -> Option<&Account> {
        self.accounts.get(account_id)
    }

    #[inline]
    pub fn get_mut(&mut self, account_id: &AccountIdRef) -> Option<&mut Account> {
        self.accounts.get_mut(account_id)
    }

    #[inline]
    pub fn get_or_create(&mut self, account_id: AccountId) -> &mut Account {
        self.accounts
            .entry(account_id)
            .or_insert_with_key(|account_id| {
                Account::new(
                    self.prefix
                        .as_slice()
                        .nest(AccountsPrefix::Account(account_id)),
                    account_id,
                )
            })
    }
}

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "::near_sdk::borsh")]
enum AccountsPrefix<'a> {
    Accounts,
    Account(&'a AccountId),
}

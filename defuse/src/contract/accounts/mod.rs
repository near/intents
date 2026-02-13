mod account;
mod force;
mod state;

pub use self::{account::*, state::*};

use std::{borrow::Cow, collections::HashSet};

use defuse_core::{
    DefuseError, Nonce, Result,
    accounts::{AccountEvent, PublicKeyEvent},
    crypto::PublicKey,
    engine::{State, StateView},
    events::DefuseEvent,
    intents::{MaybeIntentEvent, account::SetAuthByPredecessorId},
};

use defuse_near_utils::{Lock, NestPrefix, UnwrapOrPanic};
use defuse_serde_utils::base64::AsBase64;

use near_sdk::{
    AccountId, AccountIdRef, BorshStorageKey, FunctionError, IntoStorageKey, assert_one_yocto,
    borsh::BorshSerialize, env, near, store::IterableMap,
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
        let account_id = self.ensure_auth_predecessor_id();

        self.add_public_key_and_emit_event(account_id.as_ref(), public_key);
    }

    #[payable]
    fn remove_public_key(&mut self, public_key: PublicKey) {
        assert_one_yocto();
        let account_id = self.ensure_auth_predecessor_id();

        self.remove_public_key_and_emit_event(account_id.as_ref(), public_key);
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

        self.set_auth_by_predecessor_id_and_emit_event(
            &self.ensure_auth_predecessor_id(),
            false,
            false,
        )
        .unwrap_or_panic();
    }
}

impl Contract {
    #[inline]
    pub fn ensure_auth_predecessor_id(&self) -> AccountId {
        let predecessor_account_id = env::predecessor_account_id();
        if !StateView::is_auth_by_predecessor_id_enabled(self, &predecessor_account_id) {
            DefuseError::AuthByPredecessorIdDisabled(predecessor_account_id).panic();
        }
        predecessor_account_id
    }

    pub fn set_auth_by_predecessor_id_and_emit_event(
        &mut self,
        account_id: &AccountIdRef,
        enable: bool,
        force: bool,
    ) -> Result<bool> {
        let toggled = self.internal_set_auth_by_predecessor_id(account_id, enable, force)?;

        if toggled {
            DefuseEvent::SetAuthByPredecessorId(MaybeIntentEvent::new(AccountEvent::new(
                Cow::Borrowed(account_id),
                Cow::Owned(SetAuthByPredecessorId { enabled: enable }),
            )))
            .emit();
        }

        Ok(toggled)
    }

    pub(crate) fn internal_set_auth_by_predecessor_id(
        &mut self,
        account_id: &AccountIdRef,
        enable: bool,
        force: bool,
    ) -> Result<bool> {
        if enable {
            let Some(account) = self.accounts.get_mut(account_id) else {
                // no need to create an account: not-yet-existing accounts
                // have auth by PREDECESSOR_ID enabled by default
                return Ok(true);
            };
            account
        } else {
            self.accounts.get_or_create(account_id.into())
        }
        .get_mut_maybe_forced(force)
        .ok_or_else(|| DefuseError::AccountLocked(account_id.into()))
        .map(|account| account.set_auth_by_predecessor_id(enable))
    }

    pub fn add_public_key_and_emit_event(
        &mut self,
        account_id: &AccountIdRef,
        public_key: PublicKey,
    ) {
        State::add_public_key(self, account_id.into(), public_key).unwrap_or_panic();

        DefuseEvent::PublicKeyAdded(MaybeIntentEvent::new(AccountEvent::new(
            Cow::Borrowed(account_id),
            PublicKeyEvent {
                public_key: Cow::Borrowed(&public_key),
            },
        )))
        .emit();
    }

    pub fn remove_public_key_and_emit_event(
        &mut self,
        account_id: &AccountIdRef,
        public_key: PublicKey,
    ) {
        State::remove_public_key(self, account_id.into(), public_key).unwrap_or_panic();

        DefuseEvent::PublicKeyRemoved(MaybeIntentEvent::new(AccountEvent::new(
            Cow::Borrowed(account_id),
            PublicKeyEvent {
                public_key: Cow::Borrowed(&public_key),
            },
        )))
        .emit();
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

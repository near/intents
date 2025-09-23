use defuse_bitmap::{U248, U256};
use defuse_near_utils::NestPrefix;
use impl_tools::autoimpl;
use near_sdk::{
    AccountIdRef, near,
    store::{IterableSet, LookupMap},
};

use defuse_core::{
    Nonces, Result,
    accounts::{AccountEvent, PublicKeyEvent},
    crypto::PublicKey,
    events::DefuseEvent,
};
use std::borrow::Cow;

use crate::contract::accounts::{
    Account, AccountState, MaybeLegacyAccountNonces,
    account::{AccountFlags, AccountPrefix},
};

/// Legacy: V0 of [`Account`]
#[derive(Debug)]
#[near(serializers = [borsh])]
#[autoimpl(Deref using self.state)]
#[autoimpl(DerefMut using self.state)]
pub struct AccountV0 {
    pub(super) nonces: Nonces<LookupMap<U248, U256>>,

    pub(super) implicit_public_key_removed: bool,
    pub(super) public_keys: IterableSet<PublicKey>,

    pub state: AccountState,

    pub(super) prefix: Vec<u8>,
}

impl From<AccountV0> for Account {
    fn from(
        AccountV0 {
            nonces,
            implicit_public_key_removed,
            public_keys,
            state,
            prefix,
        }: AccountV0,
    ) -> Self {
        Self {
            nonces: MaybeLegacyAccountNonces::with_legacy(
                prefix.as_slice().nest(AccountPrefix::OptimizedNonces),
                nonces,
            ),
            flags: implicit_public_key_removed
                .then_some(AccountFlags::IMPLICIT_PUBLIC_KEY_REMOVED)
                .unwrap_or_else(AccountFlags::empty),
            public_keys,
            state,
            prefix,
        }
    }
}

/// Legacy implementation of [`AccountV0`]
impl AccountV0 {
    #[cfg(test)]
    #[inline]
    pub fn new<S>(prefix: S, me: &AccountIdRef) -> Self
    where
        S: near_sdk::IntoStorageKey,
    {
        let prefix = prefix.into_storage_key();

        Self {
            nonces: Nonces::new(LookupMap::new(
                prefix.as_slice().nest(AccountPrefix::_LegacyNonces),
            )),
            implicit_public_key_removed: !me.get_account_type().is_implicit(),
            public_keys: IterableSet::new(prefix.as_slice().nest(AccountPrefix::PublicKeys)),
            state: AccountState::new(prefix.as_slice().nest(AccountPrefix::State)),
            prefix,
        }
    }

    #[inline]
    #[must_use]
    pub fn add_public_key(&mut self, me: &AccountIdRef, public_key: PublicKey) -> bool {
        if !self.maybe_add_public_key(me, public_key) {
            return false;
        }

        DefuseEvent::PublicKeyAdded(AccountEvent::new(
            Cow::Borrowed(me),
            PublicKeyEvent {
                public_key: Cow::Borrowed(&public_key),
            },
        ))
        .emit();

        true
    }

    #[inline]
    #[must_use]
    fn maybe_add_public_key(&mut self, me: &AccountIdRef, public_key: PublicKey) -> bool {
        if me == public_key.to_implicit_account_id() {
            let was_removed = self.implicit_public_key_removed;
            self.implicit_public_key_removed = false;
            was_removed
        } else {
            self.public_keys.insert(public_key)
        }
    }

    #[inline]
    #[must_use]
    pub fn remove_public_key(&mut self, me: &AccountIdRef, public_key: &PublicKey) -> bool {
        if !self.maybe_remove_public_key(me, public_key) {
            return false;
        }

        DefuseEvent::PublicKeyRemoved(AccountEvent::new(
            Cow::Borrowed(me),
            PublicKeyEvent {
                public_key: Cow::Borrowed(public_key),
            },
        ))
        .emit();

        true
    }

    #[inline]
    #[must_use]
    fn maybe_remove_public_key(&mut self, me: &AccountIdRef, public_key: &PublicKey) -> bool {
        if me == public_key.to_implicit_account_id() {
            let was_removed = self.implicit_public_key_removed;
            self.implicit_public_key_removed = true;
            !was_removed
        } else {
            self.public_keys.remove(public_key)
        }
    }

    #[inline]
    pub fn commit_nonce(&mut self, n: U256) -> Result<()> {
        self.nonces.commit(n)
    }
}

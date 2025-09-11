mod entry;

pub use self::entry::*;

use std::borrow::Cow;

use bitflags::bitflags;
use defuse_bitmap::{U248, U256};
use defuse_core::{
    Nonces, Result,
    accounts::{AccountEvent, PublicKeyEvent},
    crypto::PublicKey,
    events::DefuseEvent,
    intents::account::SetAuthByPredecessorId,
};

use defuse_near_utils::NestPrefix;
use impl_tools::autoimpl;
use near_sdk::{
    AccountIdRef, BorshStorageKey, IntoStorageKey,
    borsh::BorshSerialize,
    near,
    store::{IterableSet, LookupMap},
};

use super::AccountState;

// NOTE: in order to migrate to a new version (even when adding new fields),
// see docs for `VersionedAccountEntry`
#[derive(Debug)]
#[near(serializers = [borsh])]
#[autoimpl(Deref using self.state)]
#[autoimpl(DerefMut using self.state)]
pub struct Account {
    nonces: Nonces<LookupMap<U248, U256>>,

    flags: AccountFlags,
    public_keys: IterableSet<PublicKey>,

    pub state: AccountState,

    prefix: Vec<u8>,
}

impl Account {
    #[inline]
    pub fn new<S>(prefix: S, me: &AccountIdRef) -> Self
    where
        S: IntoStorageKey,
    {
        let prefix = prefix.into_storage_key();

        Self {
            nonces: Nonces::new(LookupMap::new(
                prefix.as_slice().nest(AccountPrefix::Nonces),
            )),
            flags: (!me.get_account_type().is_implicit())
                .then_some(AccountFlags::IMPLICIT_PUBLIC_KEY_REMOVED)
                .unwrap_or_else(AccountFlags::empty),
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
            let was_removed = self.is_implicit_public_key_removed();
            self.set_implicit_public_key_removed(false);
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
            let was_removed = self.is_implicit_public_key_removed();
            self.set_implicit_public_key_removed(true);
            !was_removed
        } else {
            self.public_keys.remove(public_key)
        }
    }

    #[inline]
    pub fn has_public_key(&self, me: &AccountIdRef, public_key: &PublicKey) -> bool {
        !self.is_implicit_public_key_removed() && me == public_key.to_implicit_account_id()
            || self.public_keys.contains(public_key)
    }

    #[inline]
    pub fn iter_public_keys(&self, me: &AccountIdRef) -> impl Iterator<Item = PublicKey> + '_ {
        self.public_keys.iter().copied().chain(
            (!self.is_implicit_public_key_removed())
                .then(|| PublicKey::from_implicit_account_id(me))
                .flatten(),
        )
    }

    #[inline]
    pub fn is_nonce_used(&self, nonce: U256) -> bool {
        self.nonces.is_used(nonce)
    }

    #[inline]
    pub fn commit_nonce(&mut self, n: U256) -> Result<()> {
        self.nonces.commit(n)
    }

    /// Clears the nonce if it was expired.
    /// Returns whether the nonces was cleared. If the nonce has not expired yet, then returns `false`,
    /// regardless of whether it was previously committed or not.
    #[inline]
    pub fn clear_expired_nonce(&mut self, n: U256) -> bool {
        self.nonces.clear_expired(n)
    }

    #[inline]
    const fn is_implicit_public_key_removed(&self) -> bool {
        self.flags
            .contains(AccountFlags::IMPLICIT_PUBLIC_KEY_REMOVED)
    }

    #[inline]
    fn set_implicit_public_key_removed(&mut self, removed: bool) {
        self.flags
            .set(AccountFlags::IMPLICIT_PUBLIC_KEY_REMOVED, removed);
    }

    /// Returns whether authentication by PREDECESSOR is enabled.
    pub const fn is_auth_by_predecessor_id_enabled(&self) -> bool {
        !self
            .flags
            .contains(AccountFlags::AUTH_BY_PREDECESSOR_ID_DISABLED)
    }

    /// Sets whether authentication by `PREDECESSOR_ID` is enabled.
    /// Returns whether authentication by `PREDECESSOR_ID` was enabled
    /// before.
    pub fn set_auth_by_predecessor_id(&mut self, me: &AccountIdRef, enable: bool) -> bool {
        let was_enabled = self.is_auth_by_predecessor_id_enabled();
        let toggle = was_enabled ^ enable;
        if toggle {
            self.flags
                .toggle(AccountFlags::AUTH_BY_PREDECESSOR_ID_DISABLED);

            DefuseEvent::SetAuthByPredecessorId(AccountEvent::new(
                Cow::Borrowed(me),
                SetAuthByPredecessorId { enabled: enable },
            ))
            .emit();
        }
        was_enabled
    }
}

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "::near_sdk::borsh")]
enum AccountPrefix {
    Nonces,
    PublicKeys,
    State,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[near(serializers = [borsh])]
#[repr(transparent)]
struct AccountFlags(u8);

bitflags! {
    impl AccountFlags: u8 {
        // It was a legacy `implicit_public_key_removed: bool`
        // flag in previous version. It's safe to migrate here,
        // since borsh serializes `bool` to 0u8/1u8
        const IMPLICIT_PUBLIC_KEY_REMOVED     = 1 << 0;
        const AUTH_BY_PREDECESSOR_ID_DISABLED = 1 << 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use near_sdk::borsh;
    use rstest::rstest;

    #[rstest]
    #[test]
    fn upgrade_to_flags(#[values(true, false)] implicit_public_key_removed: bool) {
        let serialized_legacy = borsh::to_vec(&implicit_public_key_removed).unwrap();
        let flags: AccountFlags = borsh::from_slice(&serialized_legacy).unwrap();
        assert_eq!(
            flags.contains(AccountFlags::IMPLICIT_PUBLIC_KEY_REMOVED),
            implicit_public_key_removed,
            "implicit_public_key_removed doesn't match"
        );
        assert_eq!(
            borsh::to_vec(&flags).unwrap(),
            serialized_legacy,
            "unknown flags set"
        );
    }
}

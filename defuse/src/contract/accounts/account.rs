use std::{borrow::Cow, io};

use bitflags::bitflags;
use defuse_bitmap::{U248, U256};
use defuse_borsh_utils::r#as::{As, BorshDeserializeAs, BorshSerializeAs, FromInto, Or, Same};
use defuse_core::{
    Nonces,
    accounts::{AccountEvent, PublicKeyEvent},
    crypto::PublicKey,
    events::DefuseEvent,
    intents::account::SetAuthByPredecessorId,
};
use defuse_near_utils::{Lock, NestPrefix, PanicOnClone};
use impl_tools::autoimpl;
use near_sdk::{
    AccountIdRef, BorshStorageKey, IntoStorageKey,
    borsh::BorshSerialize,
    near,
    store::{IterableSet, LookupMap},
};

use super::AccountState;

#[derive(Debug)]
#[autoimpl(Deref using self.0)]
#[autoimpl(DerefMut using self.0)]
#[autoimpl(AsRef using self.0)]
#[autoimpl(AsMut using self.0)]
#[near(serializers = [borsh])]
pub struct AccountEntry(
    #[borsh(
        deserialize_with = "As::<VersionedAccountEntry>::deserialize",
        serialize_with = "As::<VersionedAccountEntry>::serialize"
    )]
    pub Lock<Account>,
);

impl From<Lock<Account>> for AccountEntry {
    fn from(account: Lock<Account>) -> Self {
        Self(account)
    }
}

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
    fn maybe_add_public_key(&mut self, me: &AccountIdRef, public_key: PublicKey) -> bool {
        if me == public_key.to_implicit_account_id() {
            let was_removed = self.implicit_public_key_removed();
            self.flags.remove(AccountFlags::IMPLICIT_PUBLIC_KEY_REMOVED);
            was_removed
        } else {
            self.public_keys.insert(public_key)
        }
    }

    #[inline]
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
    fn maybe_remove_public_key(&mut self, me: &AccountIdRef, public_key: &PublicKey) -> bool {
        if me == public_key.to_implicit_account_id() {
            let was_removed = self.implicit_public_key_removed();
            self.flags.insert(AccountFlags::IMPLICIT_PUBLIC_KEY_REMOVED);
            !was_removed
        } else {
            self.public_keys.remove(public_key)
        }
    }

    #[inline]
    pub fn has_public_key(&self, me: &AccountIdRef, public_key: &PublicKey) -> bool {
        !self.implicit_public_key_removed() && me == public_key.to_implicit_account_id()
            || self.public_keys.contains(public_key)
    }

    #[inline]
    pub fn iter_public_keys(&self, me: &AccountIdRef) -> impl Iterator<Item = PublicKey> + '_ {
        self.public_keys.iter().copied().chain(
            (!self.implicit_public_key_removed())
                .then(|| PublicKey::from_implicit_account_id(me))
                .flatten(),
        )
    }

    #[inline]
    pub fn is_nonce_used(&self, nonce: U256) -> bool {
        self.nonces.is_used(nonce)
    }

    #[inline]
    pub fn commit_nonce(&mut self, n: U256) -> bool {
        self.nonces.commit(n)
    }

    #[inline]
    const fn implicit_public_key_removed(&self) -> bool {
        self.flags
            .contains(AccountFlags::IMPLICIT_PUBLIC_KEY_REMOVED)
    }

    /// Returns whether authentication by PREDECESSOR is disabled.
    pub const fn is_auth_by_predecessor_id_disabled(&self) -> bool {
        self.flags
            .contains(AccountFlags::AUTH_BY_PREDECESSOR_ID_DISABLED)
    }

    /// Sets whether authentication by `PREDECESSOR_ID` is enabled.
    /// Returns whether authentication by `PREDECESSOR_ID` was enabled
    /// before.
    pub fn set_auth_by_predecessor_id(&mut self, me: &AccountIdRef, enable: bool) -> bool {
        let was_disabled = self.is_auth_by_predecessor_id_disabled();
        let toggle = was_disabled ^ !enable;
        if toggle {
            self.flags
                .toggle(AccountFlags::AUTH_BY_PREDECESSOR_ID_DISABLED);

            DefuseEvent::SetAuthByPredecessorId(AccountEvent::new(
                Cow::Borrowed(me),
                SetAuthByPredecessorId { enable },
            ))
            .emit();
        }
        !was_disabled
    }
}

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "::near_sdk::borsh")]
enum AccountPrefix {
    Nonces,
    PublicKeys,
    State,
}

/// Versioned [Account] state for de/serialization
#[derive(Debug)]
#[near(serializers = [borsh])]
enum VersionedAccountEntry<'a> {
    V1(Cow<'a, PanicOnClone<Account>>),
    V2(Cow<'a, PanicOnClone<Lock<Account>>>),
}

impl From<Account> for VersionedAccountEntry<'_> {
    fn from(value: Account) -> Self {
        Self::V1(Cow::Owned(value.into()))
    }
}

impl BorshDeserializeAs<Lock<Account>> for VersionedAccountEntry<'_> {
    fn deserialize_as<R>(reader: &mut R) -> io::Result<Lock<Account>>
    where
        R: io::Read,
    {
        Or::<
            // first, try to deserialize as versioned
            Same,
            // otherwise, try legacy
            FromInto<Account>,
        >::deserialize_as(reader)
        .map(|entry: Self| match entry {
            VersionedAccountEntry::V1(account) => account.into_owned().into_inner().into(),
            VersionedAccountEntry::V2(account) => account.into_owned().into_inner(),
        })
    }
}

impl BorshSerializeAs<Lock<Account>> for VersionedAccountEntry<'_> {
    fn serialize_as<W>(source: &Lock<Account>, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        // always serialize as latest version
        VersionedAccountEntry::V2(Cow::Borrowed(PanicOnClone::from_ref(source))).serialize(writer)
    }
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
    use defuse_core::tokens::TokenId;
    use near_sdk::borsh;
    use rstest::rstest;

    use super::*;

    #[test]
    fn legacy_upgrade() {
        const ACCOUNT_ID: &AccountIdRef = AccountIdRef::new_or_panic("test.near");
        let ft = TokenId::Nep141("wrap.near".parse().unwrap());

        let serialized_legacy = {
            let mut legacy = Account::new(0, ACCOUNT_ID);
            legacy.token_balances.add(ft.clone(), 123).unwrap();
            borsh::to_vec(&legacy).expect("unable to serialize legacy Account")
        };

        let serialized_versioned = {
            let mut versioned: AccountEntry = borsh::from_slice(&serialized_legacy).unwrap();
            assert_eq!(
                versioned
                    .lock()
                    .expect("legacy accounts must be unlocked by default")
                    .token_balances
                    .amount_for(&ft),
                123,
            );
            borsh::to_vec(&versioned).expect("unale to serialize versioned account")
        };

        {
            let versioned: AccountEntry = borsh::from_slice(&serialized_versioned).unwrap();
            assert_eq!(
                versioned
                    .as_locked()
                    .expect("should be locked by now")
                    .token_balances
                    .amount_for(&ft),
                123,
            );
        }
    }

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

use std::{borrow::Cow, io};

use defuse_bitmap::{U248, U256};
use defuse_borsh_utils::r#as::{BorshDeserializeAs, FromInto, Or};
use defuse_core::{
    Nonces,
    accounts::{AccountEvent, PublicKeyEvent},
    crypto::PublicKey,
    events::DefuseEvent,
};
use defuse_near_utils::{Lock, NestPrefix};
use derive_more::derive::From;
use impl_tools::autoimpl;
use near_sdk::{
    AccountIdRef, BorshStorageKey, IntoStorageKey,
    borsh::{BorshDeserialize, BorshSerialize},
    near,
    store::{IterableSet, LookupMap},
};

use super::AccountState;

#[derive(Debug)]
#[autoimpl(Deref using self.0)]
#[autoimpl(DerefMut using self.0)]
pub struct MaybeLockedAccount(Lock<Account>);

impl MaybeLockedAccount {
    // TODO
    // pub fn as_unlocked_or(&self) ->
}

#[derive(Debug)]
#[near(serializers = [borsh])]
#[autoimpl(Deref using self.state)]
#[autoimpl(DerefMut using self.state)]
pub struct Account {
    nonces: Nonces<LookupMap<U248, U256>>,

    implicit_public_key_removed: bool,
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
            implicit_public_key_removed: !me.get_account_type().is_implicit(),
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
            let was_removed = self.implicit_public_key_removed;
            self.implicit_public_key_removed = false;
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
            let was_removed = self.implicit_public_key_removed;
            self.implicit_public_key_removed = true;
            !was_removed
        } else {
            self.public_keys.remove(public_key)
        }
    }

    #[inline]
    pub fn has_public_key(&self, me: &AccountIdRef, public_key: &PublicKey) -> bool {
        !self.implicit_public_key_removed && me == public_key.to_implicit_account_id()
            || self.public_keys.contains(public_key)
    }

    #[inline]
    pub fn iter_public_keys(&self, me: &AccountIdRef) -> impl Iterator<Item = PublicKey> + '_ {
        self.public_keys.iter().copied().chain(
            (!self.implicit_public_key_removed)
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
}

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "::near_sdk::borsh")]
enum AccountPrefix {
    Nonces,
    PublicKeys,
    State,
}

#[derive(Debug, From)]
#[near(serializers = [borsh])]
enum BorshableAccount {
    V1(Account),
    V2(Lock<Account>),
}

// TODO: docs
#[derive(Debug, BorshSerialize)]
#[borsh(crate = "::near_sdk::borsh")]
enum BorshableAccountRef<'a> {
    #[allow(dead_code)]
    V1(&'a Account),
    V2(&'a Lock<Account>),
}

impl BorshDeserialize for MaybeLockedAccount {
    #[inline]
    fn deserialize_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        Or::<FromInto<BorshableAccount>, FromInto<Account>>::deserialize_as(reader)
    }
}

impl BorshSerialize for MaybeLockedAccount {
    #[inline]
    fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        BorshableAccountRef::from(self).serialize(writer)
    }
}

impl From<BorshableAccount> for MaybeLockedAccount {
    #[inline]
    fn from(account: BorshableAccount) -> Self {
        match account {
            BorshableAccount::V1(account) => account.into(),
            BorshableAccount::V2(account) => account.into(),
        }
    }
}

impl From<Account> for MaybeLockedAccount {
    #[inline]
    fn from(account: Account) -> Self {
        Self(Lock::unlocked(account))
    }
}

impl From<Lock<Account>> for MaybeLockedAccount {
    #[inline]
    fn from(account: Lock<Account>) -> Self {
        Self(account)
    }
}

impl<'a> From<&'a MaybeLockedAccount> for BorshableAccountRef<'a> {
    fn from(account: &'a MaybeLockedAccount) -> Self {
        Self::V2(&account.0)
    }
}

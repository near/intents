mod v0;
mod v1;

use std::{
    borrow::Cow,
    io::{self, Read},
    mem::size_of,
};

use defuse_borsh_utils::adapters::{As, BorshDeserializeAs, BorshSerializeAs};

use defuse_near_utils::{Lock, PanicOnClone};
use impl_tools::autoimpl;
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    near,
};

use crate::contract::accounts::account::entry::{v0::AccountV0, v1::AccountV1};

use super::Account;

#[derive(Debug)]
#[autoimpl(Deref using self.0)]
#[autoimpl(DerefMut using self.0)]
#[autoimpl(AsRef using self.0)]
#[autoimpl(AsMut using self.0)]
#[near(serializers = [borsh])]
#[repr(transparent)]
pub struct AccountEntry(
    #[borsh(
        deserialize_with = "As::<MaybeVersionedAccountEntry>::deserialize",
        serialize_with = "As::<MaybeVersionedAccountEntry>::serialize"
    )]
    pub Lock<Account>,
);

impl From<Lock<Account>> for AccountEntry {
    #[inline]
    fn from(value: Lock<Account>) -> Self {
        Self(value)
    }
}

/// Versioned [Account] state for de/serialization.
#[derive(Debug)]
#[near(serializers = [borsh])]
enum VersionedAccountEntry<'a> {
    V0(Cow<'a, PanicOnClone<AccountV0>>),
    V1(Cow<'a, PanicOnClone<Lock<AccountV1>>>),
    // When upgrading to a new version, given current version `N`:
    // 1. Copy current `Account` struct definition and name it `AccountVN`
    // 2. Add variant `VN(Cow<'a, PanicOnClone<Lock<AccountVN>>>)` before `Latest`
    // 3. Handle new variant in `match` expessions below
    // 4. Add tests for `VN -> Latest` migration
    Latest(Cow<'a, PanicOnClone<Lock<Account>>>),
}

impl From<VersionedAccountEntry<'_>> for Lock<Account> {
    fn from(versioned: VersionedAccountEntry<'_>) -> Self {
        // Borsh always deserializes into `Cow::Owned`, so it's
        // safe to call `Cow::<PanicOnClone<_>>::into_owned()` here.
        match versioned {
            VersionedAccountEntry::V0(account) => {
                Self::unlocked(account.into_owned().into_inner().into())
            }
            VersionedAccountEntry::V1(account) => account
                .into_owned()
                .into_inner()
                .map_inner_unchecked(Into::into),
            VersionedAccountEntry::Latest(account) => account
                .into_owned()
                .into_inner()
                .map_inner_unchecked(Into::into),
        }
    }
}

// Used for current accounts serialization
impl<'a> From<&'a Lock<Account>> for VersionedAccountEntry<'a> {
    fn from(value: &'a Lock<Account>) -> Self {
        // always serialize as latest version
        Self::Latest(Cow::Borrowed(PanicOnClone::from_ref(value)))
    }
}

// Used for legacy accounts deserialization
impl From<AccountV0> for VersionedAccountEntry<'_> {
    fn from(value: AccountV0) -> Self {
        Self::V0(Cow::Owned(value.into()))
    }
}

struct MaybeVersionedAccountEntry;

impl MaybeVersionedAccountEntry {
    /// This is a magic number that is used to differentiate between
    /// borsh-serialized representations of legacy and versioned [`Account`]s:
    /// * versioned [`Account`]s always start with this prefix
    /// * legacy [`Account`] starts with other 4 bytes
    ///
    /// This is safe to assume that legacy [`Account`] doesn't start with
    /// this prefix, since the first 4 bytes in legacy [`Account`] were used
    /// to denote the length of `prefix: Box<[u8]>` in [`LookupMap`] for
    /// `nonces`. Given that the original prefix is reused for other fields of
    /// [`Account`] for creating other nested prefixes, then the length of
    /// this prefix can't be the maximum of what `Box<[u8]>` can be
    /// serialized to.
    const VERSIONED_MAGIC_PREFIX: u32 = u32::MAX;
}

impl BorshDeserializeAs<Lock<Account>> for MaybeVersionedAccountEntry {
    fn deserialize_as<R>(reader: &mut R) -> io::Result<Lock<Account>>
    where
        R: io::Read,
    {
        // There will always be 4 bytes for u32:
        // * either `VERSIONED_MAGIC_PREFIX`,
        // * or u32 for `Account.nonces.prefix`
        let mut buf = [0u8; size_of::<u32>()];
        reader.read_exact(&mut buf)?;
        let prefix = u32::deserialize_reader(&mut buf.as_slice())?;

        if prefix == Self::VERSIONED_MAGIC_PREFIX {
            VersionedAccountEntry::deserialize_reader(reader)
        } else {
            // legacy account
            AccountV0::deserialize_reader(
                // prepend already consumed part of the reader
                &mut buf.chain(reader),
            )
            .map(Into::into)
        }
        .map(Into::into)
    }
}

impl<T> BorshSerializeAs<T> for MaybeVersionedAccountEntry
where
    for<'a> VersionedAccountEntry<'a>: From<&'a T>,
{
    fn serialize_as<W>(source: &T, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        (
            // always serialize as versioned and prepend magic prefix
            Self::VERSIONED_MAGIC_PREFIX,
            VersionedAccountEntry::from(source),
        )
            .serialize(writer)
    }
}

#[cfg(test)]
mod tests;

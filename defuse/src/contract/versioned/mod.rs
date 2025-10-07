mod v0;

use std::{
    borrow::Cow,
    io::{self, Read},
};

use defuse_borsh_utils::adapters::{BorshDeserializeAs, BorshSerializeAs};
use defuse_near_utils::PanicOnClone;
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    near,
};

use super::Contract;
use v0::ContractV0;

/// Versioned [Contract] state for de/serialization.
#[derive(Debug)]
#[near(serializers = [borsh])]
enum VersionedContract<'a> {
    V0(Cow<'a, PanicOnClone<ContractV0>>),
    // When upgrading to a new version, given current version `N`:
    // 1. Copy current `Contract` struct definition and name it `ContractVN`
    // 2. Add variant `VN(Cow<'a, PanicOnClone<ContractVN>>)` before `Latest`
    // 3. Handle new variant in `match` expessions below
    // 4. Add tests for `VN -> Latest` migration
    Latest(Cow<'a, PanicOnClone<Contract>>),
}

impl From<VersionedContract<'_>> for Contract {
    fn from(versioned: VersionedContract<'_>) -> Self {
        // Borsh always deserializes into `Cow::Owned`, so it's
        // safe to call `Cow::<PanicOnClone<_>>::into_owned()` here.
        match versioned {
            VersionedContract::V0(contract) => contract.into_owned().into_inner().into(),
            VersionedContract::Latest(contract) => contract.into_owned().into_inner(),
        }
    }
}

// Used for current contract serialization
impl<'a> From<&'a Contract> for VersionedContract<'a> {
    fn from(value: &'a Contract) -> Self {
        // always serialize as latest version
        Self::Latest(Cow::Borrowed(PanicOnClone::from_ref(value)))
    }
}

// Used for legacy contract deserialization
impl From<ContractV0> for VersionedContract<'_> {
    fn from(value: ContractV0) -> Self {
        Self::V0(Cow::Owned(value.into()))
    }
}

pub struct MaybeVersionedContractEntry;

impl MaybeVersionedContractEntry {
    /// This is a magic number that is used to differentiate between
    /// borsh-serialized representations of legacy and versioned [`Contract`]s:
    /// * versioned [`Contract`]s always start with this prefix
    /// * legacy [`Contract`] starts with other 4 bytes
    ///
    /// This is safe to assume that legacy [`Contract`] doesn't start with
    /// this prefix, since the first 4 bytes in legacy [`Contract`] were used
    /// to denote the length of `keys: Vector<K>,` in [`IterableMap`] for
    /// `accounts`, so coincidence can happen in case the number of accounts
    /// approaches the maximum possible, which is unlikely at this time
    /// given the number of accounts stored in the contract.
    const VERSIONED_MAGIC_PREFIX: u32 = u32::MAX;
}

impl BorshDeserializeAs<Contract> for MaybeVersionedContractEntry {
    fn deserialize_as<R>(reader: &mut R) -> io::Result<Contract>
    where
        R: io::Read,
    {
        // There will always be 4 bytes for u32:
        // * either `VERSIONED_MAGIC_PREFIX`,
        // * or u32 for `Contract.accounts.keys.len`
        let mut buf = [0u8; size_of::<u32>()];
        reader.read_exact(&mut buf)?;
        let prefix = u32::deserialize_reader(&mut buf.as_slice())?;

        if prefix == Self::VERSIONED_MAGIC_PREFIX {
            VersionedContract::deserialize_reader(reader)
        } else {
            // legacy state
            ContractV0::deserialize_reader(
                // prepend already consumed part of the reader
                &mut buf.chain(reader),
            )
            .map(Into::into)
        }
        .map(Into::into)
    }
}

impl<T> BorshSerializeAs<T> for MaybeVersionedContractEntry
where
    for<'a> VersionedContract<'a>: From<&'a T>,
{
    fn serialize_as<W>(source: &T, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        (
            // always serialize as versioned and prepend magic prefix
            Self::VERSIONED_MAGIC_PREFIX,
            VersionedContract::from(source),
        )
            .serialize(writer)
    }
}

use std::{
    borrow::Cow,
    io::{self, Read},
    mem::size_of,
};

use borsh::{BorshDeserialize, BorshSerialize};
use defuse_borsh_utils::{BorshDeserializeAs, BorshSerializeAs};
use defuse_near_utils::PanicOnClone;
use near_sdk::{
    NearToken,
    store::{IterableSet, LookupMap, LookupSet},
};

use super::{ContractStorage, Prefix};

/// Legacy (pre-omni-layer) [`ContractStorage`] shape, exactly as it is
/// currently deployed on-chain. Field order and types MUST NOT change -
/// this is what real, already-serialized state on mainnet looks like.
#[cfg_attr(feature = "abi", derive(::borsh::BorshSchema))]
#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct ContractStorageV0 {
    tokens: IterableSet<String>,
    bridge_token_storage_deposit_required: NearToken,
}

impl From<ContractStorageV0> for ContractStorage {
    fn from(old: ContractStorageV0) -> Self {
        Self {
            tokens: old.tokens,
            bridge_token_storage_deposit_required: old.bridge_token_storage_deposit_required,
            deposits: LookupSet::new(Prefix::Deposits),
            withdrawals: LookupMap::new(Prefix::Withdrawals),
            omni_tokens: IterableSet::new(Prefix::OmniTokens),
        }
    }
}

/// Versioned [`ContractStorage`] state for de/serialization.
#[cfg_attr(feature = "abi", derive(::borsh::BorshSchema))]
#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum VersionedContractStorage<'a> {
    V0(Cow<'a, PanicOnClone<ContractStorageV0>>),
    // When upgrading to a new version, given current version `N`:
    // 1. Copy current `ContractStorage` struct definition and name it `ContractStorageVN`
    // 2. Add variant `VN(Cow<'a, PanicOnClone<ContractStorageVN>>)` before `Latest`
    // 3. Handle new variant in `match` expressions below
    // 4. Add tests for `VN -> Latest` migration
    Latest(Cow<'a, PanicOnClone<ContractStorage>>),
}

impl From<VersionedContractStorage<'_>> for ContractStorage {
    fn from(versioned: VersionedContractStorage<'_>) -> Self {
        // Borsh always deserializes into `Cow::Owned`, so it's
        // safe to call `Cow::<PanicOnClone<_>>::into_owned()` here.
        match versioned {
            VersionedContractStorage::V0(storage) => storage.into_owned().into_inner().into(),
            VersionedContractStorage::Latest(storage) => storage.into_owned().into_inner(),
        }
    }
}

// Used for current contract serialization
impl<'a> From<&'a ContractStorage> for VersionedContractStorage<'a> {
    fn from(value: &'a ContractStorage) -> Self {
        // always serialize as latest version
        Self::Latest(Cow::Borrowed(PanicOnClone::from_ref(value)))
    }
}

// Used for legacy state deserialization
impl From<ContractStorageV0> for VersionedContractStorage<'_> {
    fn from(value: ContractStorageV0) -> Self {
        Self::V0(Cow::Owned(value.into()))
    }
}

pub struct MaybeVersionedContractStorage;

impl MaybeVersionedContractStorage {
    /// This is a magic number that is used to differentiate between
    /// borsh-serialized representations of legacy and versioned
    /// [`ContractStorage`]s:
    /// * versioned [`ContractStorage`]s always start with this prefix
    /// * legacy [`ContractStorageV0`] starts with other 4 bytes
    ///
    /// This is safe to assume that legacy [`ContractStorageV0`] doesn't start
    /// with this prefix, since the first 4 bytes in legacy
    /// [`ContractStorageV0`] are the length (`u32`) of the `elements` vector
    /// inside `tokens: IterableSet<String>`, i.e. the number of deployed
    /// tokens. Given the number of tokens ever deployed by this factory,
    /// this can't realistically reach `u32::MAX`.
    const VERSIONED_MAGIC_PREFIX: u32 = u32::MAX;
}

impl BorshDeserializeAs<ContractStorage> for MaybeVersionedContractStorage {
    fn deserialize_as<R>(reader: &mut R) -> io::Result<ContractStorage>
    where
        R: io::Read,
    {
        // There will always be 4 bytes for u32:
        // * either `VERSIONED_MAGIC_PREFIX`,
        // * or u32 for `ContractStorageV0.tokens.elements.len`
        let mut buf = [0u8; size_of::<u32>()];
        reader.read_exact(&mut buf)?;
        let prefix = u32::deserialize_reader(&mut buf.as_slice())?;

        if prefix == Self::VERSIONED_MAGIC_PREFIX {
            VersionedContractStorage::deserialize_reader(reader)
        } else {
            // legacy state
            ContractStorageV0::deserialize_reader(
                // prepend already consumed part of the reader
                &mut buf.chain(reader),
            )
            .map(Into::into)
        }
        .map(Into::into)
    }
}

impl<T> BorshSerializeAs<T> for MaybeVersionedContractStorage
where
    for<'a> VersionedContractStorage<'a>: From<&'a T>,
{
    fn serialize_as<W>(source: &T, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        (
            // always serialize as versioned and prepend magic prefix
            Self::VERSIONED_MAGIC_PREFIX,
            VersionedContractStorage::from(source),
        )
            .serialize(writer)
    }
}

#[cfg(test)]
mod tests;

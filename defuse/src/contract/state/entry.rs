use defuse_borsh_utils::adapters::{As, BorshDeserializeAs, BorshSerializeAs};
use defuse_core::fees::FeesConfig;
use defuse_near_utils::PanicOnClone;
use impl_tools::autoimpl;
use near_sdk::{
    IntoStorageKey,
    borsh::{BorshDeserialize, BorshSerialize},
    near,
};

use std::{
    borrow::Cow,
    io::{self, Read},
    mem::size_of,
};

use crate::contract::state::{ContractState, v0::ContractStateV0};

#[derive(Debug)]
#[autoimpl(Deref using self.0)]
#[autoimpl(DerefMut using self.0)]
#[autoimpl(AsRef using self.0)]
#[autoimpl(AsMut using self.0)]
#[near(serializers = [borsh])]
#[repr(transparent)]
pub struct ContractStateEntry(
    #[borsh(
        deserialize_with = "As::<MaybeVersionedStateEntry>::deserialize",
        serialize_with = "As::<MaybeVersionedStateEntry>::serialize"
    )]
    pub ContractState,
);

impl ContractStateEntry {
    #[inline]
    pub fn new<T>(prefix: T, wnear_id: near_sdk::AccountId, fees: FeesConfig) -> Self
    where
        T: IntoStorageKey,
    {
        Self(ContractState::new(prefix, wnear_id, fees))
    }
}

struct MaybeVersionedStateEntry;

impl MaybeVersionedStateEntry {
    /// This is a magic number that is used to differentiate between
    /// borsh-serialized representations of legacy and versioned [`ContractState`]s:
    /// * versioned [`ContractState`]s always start with this prefix
    /// * legacy [`ContractState`] starts with other 4 bytes
    ///
    /// This is safe to assume that legacy [`ContractState`] doesn't start with
    /// this prefix, since the first 4 bytes in legacy [`ContractState`] were used
    /// to denote the length of `keys: Vector<K>,` in [`IterableMap`] for
    /// `total_supplies`, so coincidence is impossible given the number of tokens
    /// stored on the contract.
    const VERSIONED_MAGIC_PREFIX: u32 = u32::MAX;
}

#[derive(Debug)]
#[near(serializers = [borsh])]
enum VersionedContractStateEntry<'a> {
    V0(Cow<'a, PanicOnClone<ContractStateV0>>),
    Latest(Cow<'a, PanicOnClone<ContractState>>),
}

impl From<VersionedContractStateEntry<'_>> for ContractState {
    fn from(versioned: VersionedContractStateEntry<'_>) -> Self {
        match versioned {
            VersionedContractStateEntry::V0(state) => state.into_owned().into_inner().into(),
            VersionedContractStateEntry::Latest(state) => state.into_owned().into_inner(),
        }
    }
}

// Used for current state serialization
impl<'a> From<&'a ContractState> for VersionedContractStateEntry<'a> {
    fn from(value: &'a ContractState) -> Self {
        // always serialize as latest version
        Self::Latest(Cow::Borrowed(PanicOnClone::from_ref(value)))
    }
}

// Used for legacy state deserialization
impl From<ContractStateV0> for VersionedContractStateEntry<'_> {
    fn from(value: ContractStateV0) -> Self {
        Self::V0(Cow::Owned(value.into()))
    }
}

impl BorshDeserializeAs<ContractState> for MaybeVersionedStateEntry {
    fn deserialize_as<R>(reader: &mut R) -> io::Result<ContractState>
    where
        R: io::Read,
    {
        // There will always be 4 bytes for u32:
        // * either `VERSIONED_MAGIC_PREFIX`,
        // * or u32 for `ContractState.total_supplies.keys.len`
        let mut buf = [0u8; size_of::<u32>()];
        reader.read_exact(&mut buf)?;
        let prefix = u32::deserialize_reader(&mut buf.as_slice())?;

        if prefix == Self::VERSIONED_MAGIC_PREFIX {
            VersionedContractStateEntry::deserialize_reader(reader)
        } else {
            // legacy state
            ContractStateV0::deserialize_reader(
                // prepend already consumed part of the reader
                &mut buf.chain(reader),
            )
            .map(Into::into)
        }
        .map(Into::into)
    }
}

impl<T> BorshSerializeAs<T> for MaybeVersionedStateEntry
where
    for<'a> VersionedContractStateEntry<'a>: From<&'a T>,
{
    fn serialize_as<W>(source: &T, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        (
            // always serialize as versioned and prepend magic prefix
            Self::VERSIONED_MAGIC_PREFIX,
            VersionedContractStateEntry::from(source),
        )
            .serialize(writer)
    }
}

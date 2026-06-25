use std::borrow::Cow;

use defuse_borsh_utils::{BorshDeserializeAs, BorshSerializeAs};
use defuse_near_utils::PanicOnClone;
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize, io},
    near,
};

#[near(serializers = [borsh])]
#[derive(Default)]
pub struct State {
    pub nonce: u128,
}

impl BorshDeserializeAs<State> for VersionedState<'_> {
    fn deserialize_as<R>(reader: &mut R) -> io::Result<State>
    where
        R: io::Read,
    {
        VersionedState::deserialize_reader(reader).map(Into::into)
    }
}

impl BorshSerializeAs<State> for VersionedState<'_> {
    fn serialize_as<W>(source: &State, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        VersionedState::from(source).serialize(writer)
    }
}

impl From<VersionedState<'_>> for State {
    fn from(state: VersionedState) -> Self {
        match state {
            VersionedState::V1(state) => state.into_owned().into_inner(),
        }
    }
}

impl<'a> From<&'a State> for VersionedState<'a> {
    fn from(value: &'a State) -> Self {
        Self::V1(Cow::Borrowed(PanicOnClone::from_ref(value)))
    }
}

#[near(serializers = [borsh])]
pub enum VersionedState<'a> {
    V1(Cow<'a, PanicOnClone<State>>),
}

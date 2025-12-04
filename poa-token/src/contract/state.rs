use defuse_borsh_utils::adapters::{BorshDeserializeAs, BorshSerializeAs};
use defuse_near_utils::PanicOnClone;
use near_contract_standards::fungible_token::{FungibleToken, metadata::FungibleTokenMetadata};
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    near,
    store::Lazy,
};
use std::{
    borrow::Cow,
    io::{self, Read},
};

pub const DEFAULT_NO_REGISTRATION: bool = true;

#[near(serializers=[borsh])]
enum VersionedState<'a> {
    V0(Cow<'a, PanicOnClone<StateV0>>),
    Latest(Cow<'a, PanicOnClone<State>>),
}

impl From<VersionedState<'_>> for State {
    fn from(versioned: VersionedState<'_>) -> Self {
        // Borsh always deserializes into `Cow::Owned`, so it's
        // safe to call `Cow::<PanicOnClone<_>>::into_owned()` here.
        match versioned {
            VersionedState::V0(Cow::Owned(contract)) => contract.into_inner().into(),
            VersionedState::Latest(Cow::Owned(contract)) => contract.into_inner(),
            _ => unreachable!("Borsh always deserializes into `Cow::Owned`"),
        }
    }
}

// Used for current contract serialization
impl<'a> From<&'a State> for VersionedState<'a> {
    fn from(value: &'a State) -> Self {
        // always serialize as latest version
        Self::Latest(Cow::Borrowed(PanicOnClone::from_ref(value)))
    }
}

// Used for legacy contract deserialization
impl From<StateV0> for VersionedState<'_> {
    fn from(value: StateV0) -> Self {
        Self::V0(Cow::Owned(value.into()))
    }
}

#[near(serializers=[borsh])]
pub struct State {
    pub token: FungibleToken,
    pub metadata: Lazy<FungibleTokenMetadata>,
    pub no_registration: bool,
}

#[near(serializers=[borsh])]
pub struct StateV0 {
    pub token: FungibleToken,
    pub metadata: Lazy<FungibleTokenMetadata>,
}

impl From<StateV0> for State {
    fn from(StateV0 { token, metadata }: StateV0) -> Self {
        Self {
            token,
            metadata,
            no_registration: DEFAULT_NO_REGISTRATION,
        }
    }
}

pub struct MaybeVersionedContractState;

impl MaybeVersionedContractState {
    /// This is a magic number that is used to differentiate between
    /// borsh-serialized representations of legacy and versioned [`Contract`]s:
    /// * versioned [`Contract`]s always start with this prefix
    /// * legacy [`Contract`] starts with other bytes
    ///
    /// This is safe to assume that legacy [`Contract`] doesn't start with
    /// this prefix, since the first 4 bytes in legacy [`Contract`] were used
    /// to denote the byte array of prefix in [`LookupMap`] for
    /// `token.accounts`, which is equal to 'Prefix::FungibleToken'
    const VERSIONED_MAGIC_PREFIX: u32 = u32::MAX;
}

impl BorshDeserializeAs<State> for MaybeVersionedContractState {
    fn deserialize_as<R>(reader: &mut R) -> io::Result<State>
    where
        R: io::Read,
    {
        // There will always be 4 bytes for u32:
        // * either `VERSIONED_MAGIC_PREFIX`,
        // * or Vec<u8> for `Contract.token.accounts.key_prefix`
        let mut buf = [0u8; size_of::<u32>()];
        reader.read_exact(&mut buf)?;
        let prefix = u32::deserialize_reader(&mut buf.as_slice())?;

        if prefix == Self::VERSIONED_MAGIC_PREFIX {
            VersionedState::deserialize_reader(reader)
        } else {
            // legacy state
            StateV0::deserialize_reader(
                // prepend already consumed part of the reader
                &mut buf.chain(reader),
            )
            .map(Into::into)
        }
        .map(Into::into)
    }
}

impl<T> BorshSerializeAs<T> for MaybeVersionedContractState
where
    for<'a> VersionedState<'a>: From<&'a T>,
{
    fn serialize_as<W>(source: &T, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        (
            // always serialize as versioned and prepend magic prefix
            Self::VERSIONED_MAGIC_PREFIX,
            VersionedState::from(source),
        )
            .serialize(writer)
    }
}

#[cfg(test)]
mod tests {
    use crate::contract::{Contract, Prefix};

    use super::*;

    use arbitrary_with::{Arbitrary, As, arbitrary};
    use defuse_near_utils::arbitrary::ArbitraryAccountId;
    use defuse_test_utils::random::make_arbitrary;
    use near_contract_standards::fungible_token::{Balance, FungibleTokenCore};
    use near_sdk::{AccountId, StorageUsage, borsh, json_types::U128};
    use rstest::rstest;

    fn deserialize_and_check_legacy_state(serialized_legacy: &[u8], data: &TokenData) {
        let mut versioned: Contract = borsh::from_slice(serialized_legacy).unwrap();

        data.assert_metadata(&versioned.state.metadata);
        data.assert_token(&versioned.state.token);

        let new_owner_id: AccountId = "new-owner.testnet".parse().unwrap();
        let amount = U128::from(1_000_000_000_000);
        versioned.token.internal_register_account(&new_owner_id);
        versioned
            .token
            .internal_deposit(&new_owner_id, amount.into());

        let serialized_versioned = borsh::to_vec(&versioned).unwrap();
        drop(versioned);

        let versioned: Contract = borsh::from_slice(&serialized_versioned).unwrap();

        data.assert_metadata(&versioned.state.metadata);
        data.assert_token(&versioned.state.token);
        assert!(versioned.ft_balance_of(new_owner_id) == amount);
    }

    #[derive(Arbitrary)]
    struct AccountData {
        #[arbitrary(with = As::<ArbitraryAccountId>::arbitrary)]
        pub account_id: AccountId,
        pub balance: Balance,
    }

    /// Data for legacy token state creation
    #[derive(Arbitrary)]
    struct TokenData {
        pub accounts: Vec<AccountData>,
        pub account_storage_usage: StorageUsage,

        pub spec: String,
        pub name: String,
        pub symbol: String,
        pub icon: Option<String>,
        pub reference: Option<String>,
        pub decimals: u8,
    }

    impl TokenData {
        pub fn create_legacy_state(&self) -> StateV0 {
            let mut token = FungibleToken::new(Prefix::FungibleToken);
            token.account_storage_usage = self.account_storage_usage;

            for account_data in &self.accounts {
                token.internal_register_account(&account_data.account_id);
                token.internal_deposit(&account_data.account_id, account_data.balance);
            }

            let metadata = FungibleTokenMetadata {
                spec: self.spec.clone(),
                name: self.name.clone(),
                symbol: self.symbol.clone(),
                icon: self.icon.clone(),
                reference: self.reference.clone(),
                reference_hash: None,
                decimals: self.decimals,
            };

            StateV0 {
                token,
                metadata: Lazy::new(Prefix::Metadata, metadata),
            }
        }

        pub fn assert_metadata(&self, metadata: &FungibleTokenMetadata) {
            assert_eq!(metadata.spec, self.spec);
            assert_eq!(metadata.name, self.name);
            assert_eq!(metadata.symbol, self.symbol);
            assert_eq!(metadata.icon, self.icon);
            assert_eq!(metadata.reference, self.reference);
            assert_eq!(metadata.decimals, self.decimals);
        }

        pub fn assert_token(&self, token: &FungibleToken) {
            assert_eq!(token.account_storage_usage, self.account_storage_usage);

            for account_data in &self.accounts {
                let balance = token.internal_unwrap_balance_of(&account_data.account_id);
                assert_eq!(balance, account_data.balance);
            }
        }
    }

    #[rstest]
    fn legacy_token_upgrade(#[from(make_arbitrary)] data: TokenData) {
        let legacy_acc = data.create_legacy_state();
        let serialized_legacy =
            borsh::to_vec(&legacy_acc).expect("unable to serialize legacy Account");

        // we need to drop it, so all collections from near-sdk flush to storage
        drop(legacy_acc);

        deserialize_and_check_legacy_state(&serialized_legacy, &data);
    }
}

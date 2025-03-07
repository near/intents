use core::{
    fmt::{self, Debug, Display},
    str::FromStr,
};
use std::{borrow::Cow, collections::BTreeMap};

use defuse_map_utils::{cleanup::DefaultMap, IterableMap};
use defuse_num_utils::{CheckedAdd, CheckedSub};
use impl_tools::autoimpl;
use near_account_id::ParseAccountError;
use near_sdk::{
    near,
    serde::{Deserializer, Serializer},
    AccountId,
};
use serde_with::{DeserializeAs, DeserializeFromStr, SerializeAs, SerializeDisplay};
use strum::{EnumDiscriminants, EnumString};
use thiserror::Error as ThisError;

#[derive(
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    EnumDiscriminants,
    SerializeDisplay,
    DeserializeFromStr,
)]
#[strum_discriminants(
    name(TokenIdType),
    derive(strum::Display, EnumString),
    strum(serialize_all = "snake_case")
)]
#[near(serializers = [borsh])]
pub enum TokenId {
    Nep141(
        /// Contract
        AccountId,
    ),
    Nep171(
        /// Contract
        AccountId,
        /// Token ID
        near_contract_standards::non_fungible_token::TokenId,
    ),
    Nep245(
        /// Contract
        AccountId,
        /// Token ID
        defuse_nep245::TokenId,
    ),
}

impl Debug for TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nep141(contract_id) => {
                write!(f, "{}:{}", TokenIdType::Nep141, contract_id)
            }
            Self::Nep171(contract_id, token_id) => {
                write!(f, "{}:{}:{}", TokenIdType::Nep171, contract_id, token_id)
            }
            Self::Nep245(contract_id, token_id) => {
                write!(f, "{}:{}:{}", TokenIdType::Nep245, contract_id, token_id)
            }
        }
    }
}

impl Display for TokenId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl FromStr for TokenId {
    type Err = ParseTokenIdError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (typ, data) = s
            .split_once(':')
            .ok_or(strum::ParseError::VariantNotFound)?;
        Ok(match typ.parse()? {
            TokenIdType::Nep141 => Self::Nep141(data.parse()?),
            TokenIdType::Nep171 => {
                let (contract_id, token_id) = data
                    .split_once(':')
                    .ok_or(strum::ParseError::VariantNotFound)?;
                Self::Nep171(contract_id.parse()?, token_id.to_string())
            }
            TokenIdType::Nep245 => {
                let (contract_id, token_id) = data
                    .split_once(':')
                    .ok_or(strum::ParseError::VariantNotFound)?;
                Self::Nep245(contract_id.parse()?, token_id.to_string())
            }
        })
    }
}

#[derive(Debug, ThisError)]
pub enum ParseTokenIdError {
    #[error("AccountId: {0}")]
    AccountId(#[from] ParseAccountError),
    #[error(transparent)]
    ParseError(#[from] strum::ParseError),
}

#[near(serializers = [borsh, json])]
#[autoimpl(Deref using self.0)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TokenAmounts<T = BTreeMap<TokenId, u128>>(T);

impl<T> TokenAmounts<T> {
    #[inline]
    pub const fn new(map: T) -> Self {
        Self(map)
    }

    #[inline]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> TokenAmounts<T>
where
    T: DefaultMap,
    T::V: Copy,
{
    #[inline]
    pub fn balance_of(&self, k: &T::K) -> T::V {
        self.0.get(k).copied().unwrap_or_default()
    }

    #[must_use]
    #[inline]
    pub fn add_balance(&mut self, k: T::K, amount: u128) -> Option<T::V>
    where
        T::V: CheckedAdd<u128>,
    {
        self.checked_apply(k, |a| a.checked_add(amount))
    }

    #[must_use]
    #[inline]
    pub fn with_add_balance(mut self, k: T::K, amount: u128) -> Option<Self>
    where
        T::V: CheckedAdd<u128>,
    {
        self.add_balance(k, amount)?;
        Some(self)
    }

    #[must_use]
    #[inline]
    pub fn with_add_balances(self, amounts: impl IntoIterator<Item = (T::K, u128)>) -> Option<Self>
    where
        T::V: CheckedAdd<u128>,
    {
        amounts.into_iter().try_fold(self, |amounts, (k, amount)| {
            amounts.with_add_balance(k, amount)
        })
    }

    #[must_use]
    #[inline]
    pub fn sub_balance(&mut self, k: T::K, amount: u128) -> Option<T::V>
    where
        T::V: CheckedSub<u128>,
    {
        self.checked_apply(k, |a| a.checked_sub(amount))
    }

    #[must_use]
    #[inline]
    pub fn with_sub_balance(mut self, k: T::K, amount: u128) -> Option<Self>
    where
        T::V: CheckedSub<u128>,
    {
        self.sub_balance(k, amount)?;
        Some(self)
    }

    #[must_use]
    #[inline]
    pub fn with_sub_balances(self, amounts: impl IntoIterator<Item = (T::K, u128)>) -> Option<Self>
    where
        T::V: CheckedSub<u128>,
    {
        amounts.into_iter().try_fold(self, |amounts, (k, amount)| {
            amounts.with_sub_balance(k, amount)
        })
    }

    #[must_use]
    #[inline]
    pub fn add_delta(&mut self, k: T::K, delta: i128) -> Option<T::V>
    where
        T::V: CheckedAdd<i128>,
    {
        self.checked_apply(k, |a| a.checked_add(delta))
    }

    #[must_use]
    #[inline]
    pub fn with_add_delta(mut self, k: T::K, delta: i128) -> Option<Self>
    where
        T::V: CheckedAdd<i128>,
    {
        self.add_delta(k, delta)?;
        Some(self)
    }

    #[must_use]
    #[inline]
    pub fn with_add_deltas(self, amounts: impl IntoIterator<Item = (T::K, i128)>) -> Option<Self>
    where
        T::V: CheckedAdd<i128>,
    {
        amounts
            .into_iter()
            .try_fold(self, |amounts, (k, delta)| amounts.with_add_delta(k, delta))
    }

    #[must_use]
    #[inline]
    fn checked_apply(&mut self, k: T::K, f: impl FnOnce(T::V) -> Option<T::V>) -> Option<T::V> {
        let mut a = self.0.entry_or_default(k);
        *a = f(*a)?;
        Some(*a)
    }
}

#[allow(clippy::iter_without_into_iter)]
impl<T> TokenAmounts<T>
where
    T: IterableMap,
{
    pub fn iter(&self) -> T::Iter<'_> {
        self.0.iter()
    }
}

impl<T> IntoIterator for TokenAmounts<T>
where
    T: IntoIterator,
{
    type Item = T::Item;

    type IntoIter = T::IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.into_inner().into_iter()
    }
}

impl<'a, T> IntoIterator for &'a TokenAmounts<T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;

    type IntoIter = <&'a T as IntoIterator>::IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T> TokenAmounts<T>
where
    T: IterableMap,
{
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<T> From<TokenAmounts<T>> for Cow<'_, TokenAmounts<T>>
where
    T: Clone,
{
    fn from(value: TokenAmounts<T>) -> Self {
        Self::Owned(value)
    }
}

impl<T, As> SerializeAs<TokenAmounts<T>> for TokenAmounts<As>
where
    As: SerializeAs<T>,
{
    #[inline]
    fn serialize_as<S>(source: &TokenAmounts<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        As::serialize_as(&source.0, serializer)
    }
}

impl<'de, T, As> DeserializeAs<'de, TokenAmounts<T>> for TokenAmounts<As>
where
    As: DeserializeAs<'de, T>,
{
    #[inline]
    fn deserialize_as<D>(deserializer: D) -> Result<TokenAmounts<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        As::deserialize_as(deserializer).map(TokenAmounts)
    }
}

#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
mod abi {
    use super::*;

    use near_sdk::schemars::{
        gen::SchemaGenerator,
        schema::{InstanceType, Schema, SchemaObject},
        JsonSchema,
    };
    use serde_with::schemars_0_8::JsonSchemaAs;

    impl JsonSchema for TokenId {
        fn schema_name() -> String {
            stringify!(TokenId).to_string()
        }

        fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
            SchemaObject {
                instance_type: Some(InstanceType::String.into()),
                extensions: [(
                    "examples",
                    [
                        Self::Nep141("ft.near".parse().unwrap()),
                        Self::Nep171("nft.near".parse().unwrap(), "token_id1".to_string()),
                        Self::Nep245("mt.near".parse().unwrap(), "token_id1".to_string()),
                    ]
                    .map(|s| s.to_string())
                    .to_vec()
                    .into(),
                )]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
                ..Default::default()
            }
            .into()
        }
    }

    impl<T, As> JsonSchemaAs<TokenAmounts<T>> for TokenAmounts<As>
    where
        As: JsonSchemaAs<T>,
    {
        fn schema_name() -> String {
            As::schema_name()
        }

        fn is_referenceable() -> bool {
            false
        }

        fn json_schema(gen: &mut SchemaGenerator) -> Schema {
            As::json_schema(gen)
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn invariant() {
        let [t1, t2] = ["t1.near", "t2.near"].map(|t| TokenId::Nep141(t.parse().unwrap()));

        assert!(TokenAmounts::<BTreeMap<TokenId, i128>>::default().is_empty());
        assert!(TokenAmounts::<BTreeMap<_, i128>>::default()
            .with_add_deltas([(t1.clone(), 0)])
            .unwrap()
            .is_empty());

        assert!(!TokenAmounts::<BTreeMap<_, i128>>::default()
            .with_add_deltas([(t1.clone(), 1)])
            .unwrap()
            .is_empty());

        assert!(!TokenAmounts::<BTreeMap<_, i128>>::default()
            .with_add_deltas([(t1.clone(), -1)])
            .unwrap()
            .is_empty());

        assert!(TokenAmounts::<BTreeMap<_, i128>>::default()
            .with_add_deltas([(t1.clone(), 1), (t1.clone(), -1)])
            .unwrap()
            .is_empty());

        assert!(!TokenAmounts::<BTreeMap<_, i128>>::default()
            .with_add_deltas([(t1.clone(), 1), (t1.clone(), -1), (t2.clone(), -1)])
            .unwrap()
            .is_empty());

        assert!(TokenAmounts::<BTreeMap<_, i128>>::default()
            .with_add_deltas([
                (t1.clone(), 1),
                (t1.clone(), -1),
                (t2.clone(), -1),
                (t2.clone(), 1)
            ])
            .unwrap()
            .is_empty());
    }
}

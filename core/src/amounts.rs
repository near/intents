use core::fmt::Debug;
use std::{borrow::Cow, collections::BTreeMap};

use defuse_map_utils::{IterableMap, cleanup::DefaultMap};
use defuse_num_utils::{CheckedAdd, CheckedSub};
use impl_tools::autoimpl;
use near_sdk::{
    near,
    serde::{Deserializer, Serializer},
};
use serde_with::{DeserializeAs, SerializeAs};

use crate::token_id::TokenId;

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[autoimpl(Deref using self.0)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Amounts<T = BTreeMap<TokenId, u128>>(T);

impl<T> Amounts<T> {
    #[inline]
    pub const fn new(map: T) -> Self {
        Self(map)
    }

    #[inline]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Amounts<T>
where
    T: DefaultMap,
    T::V: Copy,
{
    #[inline]
    pub fn amount_for(&self, k: &T::K) -> T::V {
        self.0.get(k).copied().unwrap_or_default()
    }

    #[must_use]
    #[inline]
    pub fn add(&mut self, k: T::K, amount: u128) -> Option<T::V>
    where
        T::V: CheckedAdd<u128>,
    {
        self.checked_apply(k, |a| a.checked_add(amount))
    }

    #[must_use]
    #[inline]
    pub fn with_add(mut self, k: T::K, amount: u128) -> Option<Self>
    where
        T::V: CheckedAdd<u128>,
    {
        self.add(k, amount)?;
        Some(self)
    }

    #[must_use]
    #[inline]
    pub fn with_add_many(self, amounts: impl IntoIterator<Item = (T::K, u128)>) -> Option<Self>
    where
        T::V: CheckedAdd<u128>,
    {
        amounts
            .into_iter()
            .try_fold(self, |amounts, (k, amount)| amounts.with_add(k, amount))
    }

    #[must_use]
    #[inline]
    pub fn sub(&mut self, k: T::K, amount: u128) -> Option<T::V>
    where
        T::V: CheckedSub<u128>,
    {
        self.checked_apply(k, |a| a.checked_sub(amount))
    }

    #[must_use]
    #[inline]
    pub fn with_sub(mut self, k: T::K, amount: u128) -> Option<Self>
    where
        T::V: CheckedSub<u128>,
    {
        self.sub(k, amount)?;
        Some(self)
    }

    #[must_use]
    #[inline]
    pub fn with_sub_many(self, amounts: impl IntoIterator<Item = (T::K, u128)>) -> Option<Self>
    where
        T::V: CheckedSub<u128>,
    {
        amounts
            .into_iter()
            .try_fold(self, |amounts, (k, amount)| amounts.with_sub(k, amount))
    }

    #[must_use]
    #[inline]
    pub fn apply_delta(&mut self, k: T::K, delta: i128) -> Option<T::V>
    where
        T::V: CheckedAdd<i128>,
    {
        self.checked_apply(k, |a| a.checked_add(delta))
    }

    #[must_use]
    #[inline]
    pub fn with_apply_delta(mut self, k: T::K, delta: i128) -> Option<Self>
    where
        T::V: CheckedAdd<i128>,
    {
        self.apply_delta(k, delta)?;
        Some(self)
    }

    #[must_use]
    #[inline]
    pub fn with_apply_deltas(self, amounts: impl IntoIterator<Item = (T::K, i128)>) -> Option<Self>
    where
        T::V: CheckedAdd<i128>,
    {
        amounts.into_iter().try_fold(self, |amounts, (k, delta)| {
            amounts.with_apply_delta(k, delta)
        })
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
impl<T> Amounts<T>
where
    T: IterableMap,
{
    pub fn iter(&self) -> T::Iter<'_> {
        self.0.iter()
    }
}

impl<T> IntoIterator for Amounts<T>
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

impl<'a, T> IntoIterator for &'a Amounts<T>
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

impl<T> Amounts<T>
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

impl<T> From<Amounts<T>> for Cow<'_, Amounts<T>>
where
    T: Clone,
{
    fn from(value: Amounts<T>) -> Self {
        Self::Owned(value)
    }
}

impl<T, As> SerializeAs<Amounts<T>> for Amounts<As>
where
    As: SerializeAs<T>,
{
    #[inline]
    fn serialize_as<S>(source: &Amounts<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        As::serialize_as(&source.0, serializer)
    }
}

impl<'de, T, As> DeserializeAs<'de, Amounts<T>> for Amounts<As>
where
    As: DeserializeAs<'de, T>,
{
    #[inline]
    fn deserialize_as<D>(deserializer: D) -> Result<Amounts<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        As::deserialize_as(deserializer).map(Amounts)
    }
}

#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
mod abi {
    use super::*;

    use near_sdk::schemars::{
        JsonSchema,
        r#gen::SchemaGenerator,
        schema::{InstanceType, Schema, SchemaObject},
    };
    use serde_with::schemars_0_8::JsonSchemaAs;

    impl<T, As> JsonSchemaAs<Amounts<T>> for Amounts<As>
    where
        As: JsonSchemaAs<T>,
    {
        fn schema_name() -> String {
            As::schema_name()
        }

        fn is_referenceable() -> bool {
            false
        }

        fn json_schema(generator: &mut SchemaGenerator) -> Schema {
            As::json_schema(generator)
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::token_id::nep141::Nep141TokenId;

    use super::*;

    #[test]
    fn invariant() {
        let [t1, t2] =
            ["t1.near", "t2.near"].map(|t| TokenId::Nep141(Nep141TokenId::new(t.parse().unwrap())));

        assert!(Amounts::<BTreeMap<TokenId, i128>>::default().is_empty());
        assert!(
            Amounts::<BTreeMap<_, i128>>::default()
                .with_apply_deltas([(t1.clone(), 0)])
                .unwrap()
                .is_empty()
        );

        assert!(
            !Amounts::<BTreeMap<_, i128>>::default()
                .with_apply_deltas([(t1.clone(), 1)])
                .unwrap()
                .is_empty()
        );

        assert!(
            !Amounts::<BTreeMap<_, i128>>::default()
                .with_apply_deltas([(t1.clone(), -1)])
                .unwrap()
                .is_empty()
        );

        assert!(
            Amounts::<BTreeMap<_, i128>>::default()
                .with_apply_deltas([(t1.clone(), 1), (t1.clone(), -1)])
                .unwrap()
                .is_empty()
        );

        assert!(
            !Amounts::<BTreeMap<_, i128>>::default()
                .with_apply_deltas([(t1.clone(), 1), (t1.clone(), -1), (t2.clone(), -1)])
                .unwrap()
                .is_empty()
        );

        assert!(
            Amounts::<BTreeMap<_, i128>>::default()
                .with_apply_deltas([(t1.clone(), 1), (t1, -1), (t2.clone(), -1), (t2, 1)])
                .unwrap()
                .is_empty()
        );
    }
}

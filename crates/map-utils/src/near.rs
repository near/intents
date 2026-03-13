use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    store::{
        iterable_map::{self, IterableMap},
        key::ToKey,
        lookup_map::{self, LookupMap},
    },
};

use super::{Entry, Map, OccupiedEntry, VacantEntry};

impl<K, V, H> Map for LookupMap<K, V, H>
where
    K: Ord + BorshSerialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    type K = K;

    type V = V;

    type VacantEntry<'a>
        = lookup_map::VacantEntry<'a, K, V>
    where
        Self: 'a;

    type OccupiedEntry<'a>
        = lookup_map::OccupiedEntry<'a, K, V>
    where
        Self: 'a;

    #[inline]
    fn contains_key(&self, k: &Self::K) -> bool {
        self.contains_key(k)
    }

    #[inline]
    fn get(&self, k: &Self::K) -> Option<&Self::V> {
        self.get(k)
    }

    #[inline]
    fn get_mut(&mut self, k: &Self::K) -> Option<&mut Self::V> {
        self.get_mut(k)
    }

    #[inline]
    fn insert(&mut self, k: Self::K, v: Self::V) -> Option<Self::V> {
        self.insert(k, v)
    }

    #[inline]
    fn entry(&mut self, k: Self::K) -> Entry<Self::VacantEntry<'_>, Self::OccupiedEntry<'_>> {
        match self.entry(k) {
            lookup_map::Entry::Occupied(entry) => Entry::Occupied(entry),
            lookup_map::Entry::Vacant(entry) => Entry::Vacant(entry),
        }
    }

    #[inline]
    fn remove(&mut self, k: &Self::K) -> Option<Self::V> {
        self.remove(k)
    }
}

impl<'a, K, V> VacantEntry<'a> for lookup_map::VacantEntry<'a, K, V> {
    type K = K;

    type V = V;

    #[inline]
    fn key(&self) -> &Self::K {
        self.key()
    }

    #[inline]
    fn into_key(self) -> Self::K {
        self.into_key()
    }

    #[inline]
    fn insert(self, v: Self::V) -> &'a mut Self::V {
        self.insert(v)
    }
}

impl<'a, K, V> OccupiedEntry<'a> for lookup_map::OccupiedEntry<'a, K, V> {
    type K = K;

    type V = V;

    #[inline]
    fn key(&self) -> &Self::K {
        self.key()
    }

    #[inline]
    fn get(&self) -> &Self::V {
        self.get()
    }

    #[inline]
    fn get_mut(&mut self) -> &mut Self::V {
        self.get_mut()
    }

    #[inline]
    fn into_mut(self) -> &'a mut Self::V {
        self.into_mut()
    }

    #[inline]
    fn insert(&mut self, v: Self::V) -> Self::V {
        self.insert(v)
    }

    #[inline]
    fn remove(self) -> Self::V {
        self.remove()
    }
}

impl<K, V, H> Map for IterableMap<K, V, H>
where
    K: Ord + BorshSerialize + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    type K = K;

    type V = V;

    type VacantEntry<'a>
        = iterable_map::VacantEntry<'a, K, V, H>
    where
        Self: 'a;

    type OccupiedEntry<'a>
        = iterable_map::OccupiedEntry<'a, K, V, H>
    where
        Self: 'a;

    #[inline]
    fn contains_key(&self, k: &Self::K) -> bool {
        self.contains_key(k)
    }

    #[inline]
    fn get(&self, k: &Self::K) -> Option<&Self::V> {
        self.get(k)
    }

    #[inline]
    fn get_mut(&mut self, k: &Self::K) -> Option<&mut Self::V> {
        self.get_mut(k)
    }

    #[inline]
    fn insert(&mut self, k: Self::K, v: Self::V) -> Option<Self::V> {
        self.insert(k, v)
    }

    #[inline]
    fn entry(&mut self, k: Self::K) -> Entry<Self::VacantEntry<'_>, Self::OccupiedEntry<'_>> {
        match self.entry(k) {
            iterable_map::Entry::Occupied(entry) => Entry::Occupied(entry),
            iterable_map::Entry::Vacant(entry) => Entry::Vacant(entry),
        }
    }

    #[inline]
    fn remove(&mut self, k: &Self::K) -> Option<Self::V> {
        self.remove(k)
    }
}

impl<'a, K, V, H> VacantEntry<'a> for iterable_map::VacantEntry<'a, K, V, H>
where
    K: Ord + BorshSerialize + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    type K = K;

    type V = V;

    #[inline]
    fn key(&self) -> &Self::K {
        self.key()
    }

    #[inline]
    fn into_key(self) -> Self::K {
        self.into_key()
    }

    #[inline]
    fn insert(self, v: Self::V) -> &'a mut Self::V {
        self.insert(v)
    }
}

impl<'a, K, V, H> OccupiedEntry<'a> for iterable_map::OccupiedEntry<'a, K, V, H>
where
    K: Ord + BorshSerialize + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    type K = K;

    type V = V;

    #[inline]
    fn key(&self) -> &Self::K {
        self.key()
    }

    #[inline]
    fn get(&self) -> &Self::V {
        self.get()
    }

    #[inline]
    fn get_mut(&mut self) -> &mut Self::V {
        self.get_mut()
    }

    #[inline]
    fn into_mut(self) -> &'a mut Self::V {
        self.into_mut()
    }

    #[inline]
    fn insert(&mut self, v: Self::V) -> Self::V {
        self.insert(v)
    }

    #[inline]
    fn remove(self) -> Self::V {
        self.remove()
    }
}

impl<K, V, H> crate::IterableMap for IterableMap<K, V, H>
where
    K: Ord + BorshSerialize + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    type Keys<'a>
        = iterable_map::Keys<'a, K>
    where
        Self: 'a;

    type Values<'a>
        = iterable_map::Values<'a, K, V, H>
    where
        Self: 'a;

    type ValuesMut<'a>
        = iterable_map::ValuesMut<'a, K, V, H>
    where
        Self: 'a;

    type Iter<'a>
        = iterable_map::Iter<'a, K, V, H>
    where
        Self: 'a;

    type IterMut<'a>
        = iterable_map::IterMut<'a, K, V, H>
    where
        Self: 'a;

    type Drain<'a>
        = iterable_map::Drain<'a, K, V, H>
    where
        Self: 'a;

    #[inline]
    fn len(&self) -> usize {
        self.len().try_into().unwrap_or_else(|_| unreachable!())
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    #[inline]
    fn keys(&self) -> Self::Keys<'_> {
        self.keys()
    }

    #[inline]
    fn values(&self) -> Self::Values<'_> {
        self.values()
    }

    #[inline]
    fn values_mut(&mut self) -> Self::ValuesMut<'_> {
        self.values_mut()
    }

    #[inline]
    fn iter(&self) -> Self::Iter<'_> {
        self.iter()
    }

    #[inline]
    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        self.iter_mut()
    }

    #[inline]
    fn drain(&mut self) -> Self::Drain<'_> {
        self.drain()
    }

    #[inline]
    fn clear(&mut self) {
        self.clear();
    }
}

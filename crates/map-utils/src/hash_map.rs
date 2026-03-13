use std::{
    collections::hash_map::{self, HashMap},
    hash::{BuildHasher, Hash},
};

use crate::iter::IterableMap;

use super::{Entry, Map, OccupiedEntry, VacantEntry};

impl<K, V, S> Map for HashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    type K = K;

    type V = V;

    type VacantEntry<'a>
        = hash_map::VacantEntry<'a, K, V>
    where
        Self: 'a;

    type OccupiedEntry<'a>
        = hash_map::OccupiedEntry<'a, K, V>
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
            hash_map::Entry::Occupied(entry) => Entry::Occupied(entry),
            hash_map::Entry::Vacant(entry) => Entry::Vacant(entry),
        }
    }

    #[inline]
    fn remove(&mut self, k: &Self::K) -> Option<Self::V> {
        self.remove(k)
    }
}

impl<'a, K, V> VacantEntry<'a> for hash_map::VacantEntry<'a, K, V> {
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

impl<'a, K, V> OccupiedEntry<'a> for hash_map::OccupiedEntry<'a, K, V> {
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

impl<K, V, S> IterableMap for HashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    type Keys<'a>
        = hash_map::Keys<'a, K, V>
    where
        Self: 'a;

    type Values<'a>
        = hash_map::Values<'a, K, V>
    where
        Self: 'a;

    type ValuesMut<'a>
        = hash_map::ValuesMut<'a, K, V>
    where
        Self: 'a;

    type Iter<'a>
        = hash_map::Iter<'a, K, V>
    where
        Self: 'a;

    type IterMut<'a>
        = hash_map::IterMut<'a, K, V>
    where
        Self: 'a;

    type Drain<'a>
        = hash_map::Drain<'a, K, V>
    where
        Self: 'a;

    #[inline]
    fn len(&self) -> usize {
        self.len()
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

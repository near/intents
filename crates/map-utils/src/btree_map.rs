use std::{
    collections::btree_map::{self, BTreeMap},
    mem,
};

use crate::IterableMap;

use super::{Entry, Map, OccupiedEntry, VacantEntry};

impl<K, V> Map for BTreeMap<K, V>
where
    K: Ord,
{
    type K = K;

    type V = V;

    type VacantEntry<'a>
        = btree_map::VacantEntry<'a, K, V>
    where
        Self: 'a;

    type OccupiedEntry<'a>
        = btree_map::OccupiedEntry<'a, K, V>
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
            btree_map::Entry::Occupied(entry) => Entry::Occupied(entry),
            btree_map::Entry::Vacant(entry) => Entry::Vacant(entry),
        }
    }

    #[inline]
    fn remove(&mut self, k: &Self::K) -> Option<Self::V> {
        self.remove(k)
    }
}

impl<'a, K, V> VacantEntry<'a> for btree_map::VacantEntry<'a, K, V>
where
    K: Ord,
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

impl<'a, K, V> OccupiedEntry<'a> for btree_map::OccupiedEntry<'a, K, V>
where
    K: Ord,
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

impl<K, V> IterableMap for BTreeMap<K, V>
where
    K: Eq + Ord,
{
    type Keys<'a>
        = btree_map::Keys<'a, K, V>
    where
        Self: 'a;

    type Values<'a>
        = btree_map::Values<'a, K, V>
    where
        Self: 'a;

    type ValuesMut<'a>
        = btree_map::ValuesMut<'a, K, V>
    where
        Self: 'a;

    type Iter<'a>
        = btree_map::Iter<'a, K, V>
    where
        Self: 'a;

    type IterMut<'a>
        = btree_map::IterMut<'a, K, V>
    where
        Self: 'a;

    type Drain<'a>
        = btree_map::IntoIter<K, V>
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
        mem::take(self).into_iter()
    }

    #[inline]
    fn clear(&mut self) {
        self.clear();
    }
}

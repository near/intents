mod btree_map;
pub mod cleanup;
mod hash_map;
mod iter;
#[cfg(feature = "near")]
mod near;

pub use self::iter::*;

pub trait Map {
    type K;
    type V;

    type VacantEntry<'a>: VacantEntry<'a, K = Self::K, V = Self::V>
    where
        Self: 'a;
    type OccupiedEntry<'a>: OccupiedEntry<'a, K = Self::K, V = Self::V>
    where
        Self: 'a;

    fn contains_key(&self, k: &Self::K) -> bool;
    fn get(&self, k: &Self::K) -> Option<&Self::V>;
    fn get_mut(&mut self, k: &Self::K) -> Option<&mut Self::V>;
    fn insert(&mut self, k: Self::K, v: Self::V) -> Option<Self::V>;
    fn entry(&mut self, k: Self::K) -> Entry<Self::VacantEntry<'_>, Self::OccupiedEntry<'_>>;
    fn remove(&mut self, k: &Self::K) -> Option<Self::V>;
}

#[derive(Debug)]
pub enum Entry<V, O> {
    Vacant(V),
    Occupied(O),
}

impl<'a, V, O> Entry<V, O>
where
    V: VacantEntry<'a>,
    O: OccupiedEntry<'a, K = V::K, V = V::V>,
{
    #[inline]
    pub fn key(&self) -> &V::K {
        match self {
            Self::Vacant(entry) => entry.key(),
            Self::Occupied(entry) => entry.key(),
        }
    }

    #[inline]
    pub fn or_default(self) -> &'a mut V::V
    where
        V::V: Default,
    {
        self.or_insert_with(Default::default)
    }

    #[inline]
    pub fn or_insert(self, default: V::V) -> &'a mut V::V {
        self.or_insert_with(|| default)
    }

    #[inline]
    pub fn or_insert_with(self, default: impl FnOnce() -> V::V) -> &'a mut V::V {
        self.or_insert_with_key(|_| default())
    }

    #[inline]
    pub fn or_insert_with_key(self, default: impl FnOnce(&V::K) -> V::V) -> &'a mut V::V {
        match self {
            Self::Vacant(entry) => {
                let v = default(entry.key());
                entry.insert(v)
            }
            Self::Occupied(entry) => entry.into_mut(),
        }
    }

    #[must_use]
    #[inline]
    pub fn and_modify(mut self, f: impl FnOnce(&mut V::V)) -> Self {
        if let Self::Occupied(ref mut entry) = self {
            f(entry.get_mut());
        }
        self
    }
}

pub trait VacantEntry<'a> {
    type K;
    type V;

    fn key(&self) -> &Self::K;
    fn into_key(self) -> Self::K;
    fn insert(self, v: Self::V) -> &'a mut Self::V;
}

pub trait OccupiedEntry<'a> {
    type K;
    type V;

    fn key(&self) -> &Self::K;
    fn get(&self) -> &Self::V;
    fn get_mut(&mut self) -> &mut Self::V;
    fn into_mut(self) -> &'a mut Self::V;
    fn insert(&mut self, v: Self::V) -> Self::V;
    fn remove(self) -> Self::V;
}

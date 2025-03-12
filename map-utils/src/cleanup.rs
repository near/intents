use core::{
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use impl_tools::autoimpl;

use crate::{Entry, Map, OccupiedEntry, VacantEntry};

/// A mapping where non-existing keys considered to have [`Default`] values
pub trait DefaultMap: Map<V: Default + Eq> {
    /// Get an entry at given key or [`Default`] value if the key doesn't exist.
    ///
    /// The returned entry will automatically be removed from the map if it becomes
    /// equal to [`Default`] after modifications.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::collections::HashMap;
    /// # use defuse_map_utils::cleanup::DefaultMap;
    /// let mut m = HashMap::new();
    /// *m.entry_or_default("a") += 1;
    /// assert_eq!(m.get("a"), Some(&1));
    /// *m.entry_or_default("a") -= 1;
    /// assert_eq!(m.get("a"), None);
    /// ```
    #[inline]
    fn entry_or_default(
        &mut self,
        k: Self::K,
    ) -> DefaultEntry<'_, Self::VacantEntry<'_>, Self::OccupiedEntry<'_>> {
        match self.entry(k) {
            Entry::Vacant(entry) => DefaultVacantEntry::new(entry).into(),
            Entry::Occupied(entry) => DefaultOccupiedEntry::new(entry).into(),
        }
    }
}
impl<T> DefaultMap for T where T: Map<V: Default + Eq> {}

#[autoimpl(Debug where V: Debug, V::V: Debug, O: Debug)]
pub enum DefaultEntry<'a, V, O>
where
    V: VacantEntry<'a, V: Default + Eq>,
    O: OccupiedEntry<'a, K = V::K, V = V::V>,
{
    Vacant(DefaultVacantEntry<'a, V>),
    Occupied(DefaultOccupiedEntry<'a, O>),
}

impl<'a, V, O> DefaultEntry<'a, V, O>
where
    V: VacantEntry<'a, V: Default + Eq>,
    O: OccupiedEntry<'a, K = V::K, V = V::V>,
{
    /// Get the key associated with the entry.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::collections::HashMap;
    /// # use defuse_map_utils::cleanup::DefaultMap;
    /// let mut m: HashMap<_, ()> = HashMap::new();
    /// assert_eq!(*m.entry_or_default("a").key(), "a");
    /// ```
    #[inline]
    pub fn key(&self) -> &V::K {
        match self {
            Self::Vacant(entry) => entry.key(),
            Self::Occupied(entry) => entry.key(),
        }
    }

    /// Remove the entry from the map, regardless of its value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::collections::HashMap;
    /// # use defuse_map_utils::cleanup::DefaultMap;
    /// let mut m: HashMap<_, i32> = HashMap::new();
    /// let mut entry = m.entry_or_default("a");
    /// *entry += 1;
    /// assert_eq!(entry.remove(), 1);
    /// assert_eq!(m.get("a"), None);
    /// ```
    #[inline]
    pub fn remove(self) -> V::V {
        match self {
            Self::Vacant(entry) => entry.remove(),
            Self::Occupied(entry) => entry.remove(),
        }
    }
}

impl<'a, V, O> Deref for DefaultEntry<'a, V, O>
where
    V: VacantEntry<'a, V: Default + Eq>,
    O: OccupiedEntry<'a, K = V::K, V = V::V>,
{
    type Target = V::V;

    #[inline]
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Vacant(entry) => entry,
            Self::Occupied(entry) => entry,
        }
    }
}

impl<'a, V, O> DerefMut for DefaultEntry<'a, V, O>
where
    V: VacantEntry<'a, V: Default + Eq>,
    O: OccupiedEntry<'a, K = V::K, V = V::V>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Vacant(entry) => entry,
            Self::Occupied(entry) => entry,
        }
    }
}

impl<'a, V, O> From<DefaultVacantEntry<'a, V>> for DefaultEntry<'a, V, O>
where
    V: VacantEntry<'a, V: Default + Eq>,
    O: OccupiedEntry<'a, K = V::K, V = V::V>,
{
    #[inline]
    fn from(entry: DefaultVacantEntry<'a, V>) -> Self {
        Self::Vacant(entry)
    }
}

impl<'a, V, O> From<DefaultOccupiedEntry<'a, O>> for DefaultEntry<'a, V, O>
where
    V: VacantEntry<'a, V: Default + Eq>,
    O: OccupiedEntry<'a, K = V::K, V = V::V>,
{
    #[inline]
    fn from(entry: DefaultOccupiedEntry<'a, O>) -> Self {
        Self::Occupied(entry)
    }
}

#[derive(Debug)]
pub struct DefaultVacantEntry<'a, E: 'a>(Option<(E::V, E)>)
where
    E: VacantEntry<'a, V: Default + Eq>;

impl<'a, E: 'a> DefaultVacantEntry<'a, E>
where
    E: VacantEntry<'a, V: Default + Eq>,
{
    #[inline]
    pub fn new(entry: E) -> Self {
        Self(Some((Default::default(), entry)))
    }

    #[inline]
    pub fn key(&self) -> &E::K {
        self.0.as_ref().unwrap_or_else(|| unreachable!()).1.key()
    }

    #[inline]
    pub fn remove(mut self) -> E::V {
        self.0.take().unwrap_or_else(|| unreachable!()).0
    }
}

impl<'a, E: 'a> Deref for DefaultVacantEntry<'a, E>
where
    E: VacantEntry<'a, V: Default + Eq>,
{
    type Target = E::V;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0.as_ref().unwrap_or_else(|| unreachable!()).0
    }
}

impl<'a, E: 'a> DerefMut for DefaultVacantEntry<'a, E>
where
    E: VacantEntry<'a, V: Default + Eq>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.as_mut().unwrap_or_else(|| unreachable!()).0
    }
}

impl<'a, E: 'a> Drop for DefaultVacantEntry<'a, E>
where
    E: VacantEntry<'a, V: Default + Eq>,
{
    #[inline]
    fn drop(&mut self) {
        let Some((v, entry)) = self.0.take() else {
            return;
        };
        if v != Default::default() {
            entry.insert(v);
        }
    }
}

#[derive(Debug)]
pub struct DefaultOccupiedEntry<'a, E>(Option<E>, PhantomData<&'a ()>)
where
    E: OccupiedEntry<'a, V: Default + Eq>;

impl<'a, E> DefaultOccupiedEntry<'a, E>
where
    E: OccupiedEntry<'a, V: Default + Eq>,
{
    #[inline]
    pub fn new(entry: E) -> Self {
        Self(Some(entry), PhantomData)
    }

    #[inline]
    pub fn key(&self) -> &E::K {
        self.0.as_ref().unwrap_or_else(|| unreachable!()).key()
    }

    #[inline]
    pub fn remove(mut self) -> E::V {
        self.0.take().unwrap_or_else(|| unreachable!()).remove()
    }
}

impl<'a, E> Deref for DefaultOccupiedEntry<'a, E>
where
    E: OccupiedEntry<'a, V: Default + Eq>,
{
    type Target = E::V;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap_or_else(|| unreachable!()).get()
    }
}

impl<'a, E> DerefMut for DefaultOccupiedEntry<'a, E>
where
    E: OccupiedEntry<'a, V: Default + Eq>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().unwrap_or_else(|| unreachable!()).get_mut()
    }
}

impl<'a, E> Drop for DefaultOccupiedEntry<'a, E>
where
    E: OccupiedEntry<'a, V: Default + Eq>,
{
    #[inline]
    fn drop(&mut self) {
        let Some(entry) = self.0.take() else {
            return;
        };
        if entry.get() == &Default::default() {
            entry.remove();
        }
    }
}

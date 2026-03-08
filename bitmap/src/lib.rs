mod b256;

pub use self::b256::*;

use std::ops::{RangeInclusive, Shl};

use defuse_map_utils::{IterableMap, Map};
use near_sdk::near;
use num_traits::{One, PrimInt, Unsigned, Zero};

/// Bitmap of values `T` stored inline.
// #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct BitMap<M>(M);

impl<M> BitMap<M>
where
    M: Map<K = <M as Map>::V>,
    M::K: PrimInt + Unsigned + Shl<M::K, Output = M::K>,
{
    #[allow(clippy::as_conversions)]
    const BITS_FOR_BIT_POS: usize = (size_of::<M::K>() * 8).ilog2() as usize;

    #[inline]
    pub const fn new(map: M) -> Self {
        Self(map)
    }

    /// Get the bit `n`
    #[inline]
    pub fn get_bit(&self, n: M::K) -> bool {
        let (word, bit_mask) = Self::split_word_mask(n);
        let Some(bitmap) = self.0.get(&word) else {
            return false;
        };
        *bitmap & bit_mask != M::V::zero()
    }

    /// Set the bit `n` and return old value
    #[inline]
    pub fn set_bit(&mut self, n: M::K) -> bool {
        let (bitmap, mask) = self.get_mut_with_mask(n);
        let old = *bitmap & mask != M::V::zero();
        *bitmap = *bitmap | mask;
        old
    }

    /// Clear the bit `n` and return old value
    #[inline]
    pub fn clear_bit(&mut self, n: M::K) -> bool {
        let (bitmap, mask) = self.get_mut_with_mask(n);
        let old = *bitmap & mask != M::V::zero();
        *bitmap = *bitmap & !mask;
        old
    }

    /// Toggle the bit `n` and return old value
    #[inline]
    pub fn toggle_bit(&mut self, n: M::K) -> bool {
        let (bitmap, mask) = self.get_mut_with_mask(n);
        let old = *bitmap & mask != M::V::zero();
        *bitmap = *bitmap ^ mask;
        old
    }

    /// Set bit `n` to given value and return old value
    #[inline]
    pub fn set_bit_to(&mut self, n: M::K, v: bool) -> bool {
        if v {
            self.set_bit(n)
        } else {
            self.clear_bit(n)
        }
    }

    /// Iterate over set bits
    pub fn as_iter(&self) -> impl Iterator<Item = M::V> + '_
    where
        M: IterableMap,
        RangeInclusive<M::V>: Iterator<Item = M::V>,
    {
        self.0.iter().flat_map(|(prefix, bitmap)| {
            (M::V::zero()..=Self::bit_pos_mask())
                .filter(|&bit_pos| {
                    let bit_mask = M::V::one() << bit_pos;
                    *bitmap & bit_mask != M::V::zero()
                })
                .map(|bit_pos| (*prefix << Self::BITS_FOR_BIT_POS) | bit_pos)
        })
    }

    #[inline]
    fn get_mut_with_mask(&mut self, n: M::K) -> (&mut M::V, M::V) {
        let (word, bit_mask) = Self::split_word_mask(n);
        (self.0.entry(word).or_insert_with(M::V::zero), bit_mask)
    }

    /// Returns `(word, bit_pos_mask)`
    #[inline]
    fn split_word_mask(n: M::K) -> (M::K, M::V) {
        let word = n >> Self::BITS_FOR_BIT_POS;
        let bit_mask = M::V::one() << (n & Self::bit_pos_mask());
        (word, bit_mask)
    }

    #[inline]
    fn bit_pos_mask() -> M::V {
        (M::V::one() << Self::BITS_FOR_BIT_POS) - M::V::one()
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fmt::Debug};

    use rstest::rstest;

    use super::*;

    #[allow(clippy::used_underscore_binding)]
    #[rstest]
    fn test<T>(#[values(0u8, 0u16, 0u32, 0u64, 0u128)] _n: T)
    where
        T: PrimInt + Unsigned + Shl<T, Output = T>,
    {
        let mut m = BitMap::<BTreeMap<T, T>>::default();

        for n in [
            T::zero(),
            T::one(),
            T::max_value() - T::one(),
            T::max_value(),
        ] {
            assert!(!m.get_bit(n));

            assert!(!m.set_bit(n));
            assert!(m.get_bit(n));
            assert!(m.set_bit(n));
            assert!(m.get_bit(n));

            assert!(m.clear_bit(n));
            assert!(!m.get_bit(n));
            assert!(!m.clear_bit(n));
            assert!(!m.get_bit(n));
        }
    }

    #[rstest]
    fn as_iter<T>(
        #[values(
            Vec::<u8>::new(),
            vec![0u8],
            vec![3u16, 0, 2, 7, u16::MAX],
            vec![1000u32, 15, 23, 717, 999, u32::MAX],
        )]
        mut ns: Vec<T>,
    ) where
        RangeInclusive<T>: Iterator<Item = T>,
        T: PrimInt + Unsigned + Shl<T, Output = T> + Debug,
    {
        let mut m = BitMap::<BTreeMap<T, T>>::default();
        for n in &ns {
            assert!(!m.set_bit(*n));
        }
        ns.sort();
        assert_eq!(m.as_iter().collect::<Vec<_>>(), ns);
    }
}

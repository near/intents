use std::{
    collections::BTreeMap,
    fmt::Display,
    ops::{RangeInclusive, Shl},
    str::FromStr,
};

use defuse_borsh_utils::adapters::{Bits, BorshDeserializeAs, BorshSerializeAs};
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize, io},
    near,
    serde_with::DisplayFromStr,
};
use num_traits::{PrimInt, Unsigned};
use tlbits::{
    NBits, NoArgs, Same, VarLen,
    bitvec::mem::bits_of,
    de::{BitReader, BitReaderExt, BitUnpack},
    ser::{BitPack, BitWriter, BitWriterExt},
};

/// Bitmap of values `T` stored inline.
// #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[near(serializers = [json])]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct CompactBitMap<T>(
    #[serde_as(as = "BTreeMap<DisplayFromStr, DisplayFromStr>")]
    #[serde(bound(
        serialize = "T: Display",
        deserialize = "T: FromStr<Err: Display> + Ord",
    ))]
    BTreeMap<T, T>,
);

impl<T> CompactBitMap<T>
where
    T: PrimInt + Unsigned + Shl<T, Output = T>,
{
    const BITS: usize = bits_of::<T>();
    const BITS_FOR_BIT_POS: usize = Self::BITS.ilog2() as usize;
    const BITS_FOR_WORD: usize = Self::BITS - Self::BITS_FOR_BIT_POS;
    const MAX_LEN_BITS: usize = if Self::BITS_FOR_WORD < u32::BITS as usize {
        // add one, since we also need to store zero-length
        Self::BITS_FOR_WORD + 1
    } else {
        u32::BITS as usize
    };

    #[inline]
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Get the bit `n`
    #[inline]
    pub fn get_bit(&self, n: T) -> bool {
        let (word, bit_mask) = Self::split_word_mask(n);
        let Some(bitmap) = self.0.get(&word) else {
            return false;
        };
        *bitmap & bit_mask != T::zero()
    }

    /// Set the bit `n` and return old value
    #[inline]
    pub fn set_bit(&mut self, n: T) -> bool {
        let (bitmap, mask) = self.get_mut_with_mask(n);
        let old = *bitmap & mask != T::zero();
        *bitmap = *bitmap | mask;
        old
    }

    /// Clear the bit `n` and return old value
    #[inline]
    pub fn clear_bit(&mut self, n: T) -> bool {
        let (bitmap, mask) = self.get_mut_with_mask(n);
        let old = *bitmap & mask != T::zero();
        *bitmap = *bitmap & !mask;
        old
    }

    /// Toggle the bit `n` and return old value
    #[inline]
    pub fn toggle_bit(&mut self, n: T) -> bool {
        let (bitmap, mask) = self.get_mut_with_mask(n);
        let old = *bitmap & mask != T::zero();
        *bitmap = *bitmap ^ mask;
        old
    }

    /// Set bit `n` to given value and return old value
    #[inline]
    pub fn set_bit_to(&mut self, n: T, v: bool) -> bool {
        if v {
            self.set_bit(n)
        } else {
            self.clear_bit(n)
        }
    }

    /// Iterate over set bits
    pub fn as_iter(&self) -> impl Iterator<Item = T> + '_
    where
        RangeInclusive<T>: Iterator<Item = T>,
    {
        self.0.iter().flat_map(|(prefix, bitmap)| {
            (T::zero()..=Self::bit_pos_mask())
                .filter(|&bit_pos| {
                    let bit_mask = T::one() << bit_pos;
                    *bitmap & bit_mask != T::zero()
                })
                .map(|bit_pos| (*prefix << Self::BITS_FOR_BIT_POS) | bit_pos)
        })
    }

    #[inline]
    fn get_mut_with_mask(&mut self, n: T) -> (&mut T, T) {
        let (word, bit_mask) = Self::split_word_mask(n);
        (self.0.entry(word).or_insert_with(T::zero), bit_mask)
    }

    /// Returns `(word, bit_pos_mask)`
    #[inline]
    fn split_word_mask(n: T) -> (T, T) {
        let word = n >> Self::BITS_FOR_BIT_POS;
        let bit_mask = T::one() << (n & Self::bit_pos_mask());
        (word, bit_mask)
    }

    #[inline]
    fn bit_pos_mask() -> T {
        (T::one() << Self::BITS_FOR_BIT_POS) - T::one()
    }
}

impl<T> Default for CompactBitMap<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

// stable Rust doesn't allow to use generic parameters from outer item
macro_rules! impl_tlbits_for_bitmap {
    ($($t:ty),*) => {$(
        impl BitPack for CompactBitMap<$t> {
            type Args = ();

            fn pack<W>(&self, writer: &mut W, _: Self::Args) -> Result<(), W::Error>
            where
                W: BitWriter + ?Sized,
            {
                writer
                    .pack_as::<_, &VarLen<
                        BTreeMap<NBits<{ CompactBitMap::<$t>::BITS_FOR_WORD }>, Same>,
                        { CompactBitMap::<$t>::MAX_LEN_BITS },
                    >>(
                        &self.0,
                        NoArgs::EMPTY,
                    )?;
                Ok(())
            }
        }

        impl<'de> BitUnpack<'de> for CompactBitMap<$t> {
            type Args = ();

            fn unpack<R>(reader: &mut R, _: Self::Args) -> Result<Self, R::Error>
            where
                R: BitReader<'de> + ?Sized,
            {
                reader
                    .unpack_as::<_, VarLen<
                        BTreeMap<NBits<{ CompactBitMap::<$t>::BITS_FOR_WORD }>, Same>,
                        { CompactBitMap::<$t>::MAX_LEN_BITS },
                    >>(NoArgs::EMPTY)
                    .map(Self)
            }
        }
    )*};
}
impl_tlbits_for_bitmap!(u8, u16, u32, u64, u128);

impl<T> BorshSerialize for CompactBitMap<T>
where
    Self: BitPack<Args: NoArgs>,
{
    fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        <Bits as BorshSerializeAs<Self>>::serialize_as(self, writer)
    }
}

impl<T> BorshDeserialize for CompactBitMap<T>
where
    Self: for<'de> BitUnpack<'de, Args: NoArgs>,
{
    fn deserialize_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        <Bits as BorshDeserializeAs<Self>>::deserialize_as(reader)
    }
}

#[cfg(test)]
mod tests {
    use std::{fmt::Debug, ops::Range};

    use near_sdk::borsh;
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test<T>(#[values(0u8, 0u16, 0u32, 0u64, 0u128)] _n: T)
    where
        T: PrimInt + Unsigned + Shl<T, Output = T>,
    {
        let mut m = CompactBitMap::<T>::default();

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
        let mut m = CompactBitMap::<T>::default();
        for n in &ns {
            assert!(!m.set_bit(*n));
        }
        ns.sort();
        assert_eq!(m.as_iter().collect::<Vec<_>>(), ns);
    }

    #[rstest]
    fn borsh_roundtip<T>(
        #[values(
            0u8..127,
            0u16..1000,
            0u32..1000,
            0u64..1000,
            0u128..1000,
        )]
        ns: Range<T>,
    ) where
        Range<T>: Iterator<Item = T>,
        T: PrimInt + Unsigned + Shl<T, Output = T> + BorshSerialize + Debug,
        CompactBitMap<T>: BitPack<Args = ()> + for<'de> BitUnpack<'de, Args = ()>,
    {
        let mut m = CompactBitMap::<T>::default();
        for n in ns.clone() {
            let bit = if n & T::one() == T::zero() {
                n
            } else {
                T::max_value() - n
            };

            assert!(!m.set_bit(bit));

            let serialized = borsh::to_vec(&m).unwrap();
            {
                let serialized_inner = borsh::to_vec(&m.0).unwrap();
                assert!(
                    serialized.len() <= serialized_inner.len(),
                    "inefficient serialization"
                );
            }
            assert_eq!(m, borsh::from_slice(&serialized).unwrap());
        }
    }
}

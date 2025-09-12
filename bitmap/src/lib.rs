use defuse_map_utils::{IterableMap, Map};
use near_sdk::near;

pub type U256 = [u8; 32];
pub type U248 = [u8; 31];

/// 256-bit map.  
/// See [permit2 nonce schema](https://docs.uniswap.org/contracts/permit2/reference/signature-transfer#nonce-schema)
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, Default)]
pub struct BitMap256<T: Map<K = U248, V = U256>>(T);

impl<T> BitMap256<T>
where
    T: Map<K = U248, V = U256>,
{
    #[inline]
    pub const fn new(map: T) -> Self {
        Self(map)
    }

    /// Get the bit `n`
    #[inline]
    pub fn get_bit(&self, n: U256) -> bool {
        let [word_pos @ .., bit_pos] = n;
        let Some(bitmap) = self.0.get(&word_pos) else {
            return false;
        };
        let byte = bitmap[usize::from(bit_pos / 8)];
        let byte_mask = 1 << (bit_pos % 8);
        byte & byte_mask != 0
    }

    #[inline]
    fn get_mut_byte_with_mask(&mut self, n: U256) -> (&mut u8, u8) {
        let [word_pos @ .., bit_pos] = n;
        let bitmap = self.0.entry(word_pos).or_default();
        let byte = &mut bitmap[usize::from(bit_pos / 8)];
        let byte_mask = 1 << (bit_pos % 8);
        (byte, byte_mask)
    }

    #[inline]
    pub fn clear_by_prefix(&mut self, prefix: [u8; 31]) -> bool {
        self.0.remove(&prefix).is_some()
    }

    /// Set the bit `n` and return old value
    #[inline]
    pub fn set_bit(&mut self, n: U256) -> bool {
        let (byte, mask) = self.get_mut_byte_with_mask(n);
        let old = *byte & mask != 0;
        *byte |= mask;
        old
    }

    /// Clear the bit `n` and return old value
    #[inline]
    pub fn clear_bit(&mut self, n: U256) -> bool {
        let (byte, mask) = self.get_mut_byte_with_mask(n);
        let old = *byte & mask != 0;
        *byte &= !mask;
        old
    }

    /// Toggle the bit `n` and return old value
    #[inline]
    pub fn toggle_bit(&mut self, n: U256) -> bool {
        let (byte, mask) = self.get_mut_byte_with_mask(n);
        let old = *byte & mask != 0;
        *byte ^= mask;
        old
    }

    /// Set bit `n` to given value
    #[inline]
    pub fn set_bit_to(&mut self, n: U256, v: bool) -> bool {
        if v {
            self.set_bit(n)
        } else {
            self.clear_bit(n)
        }
    }

    /// Iterate over set U256
    #[inline]
    pub fn as_iter(&self) -> impl Iterator<Item = U256> + '_
    where
        T: IterableMap,
    {
        self.0.iter().flat_map(|(prefix, bitmap)| {
            (0..=u8::MAX)
                .filter(|&bit_pos| {
                    let byte = bitmap[usize::from(bit_pos / 8)];
                    let byte_mask = 1 << (bit_pos % 8);
                    byte & byte_mask != 0
                })
                .map(|bit_pos| {
                    let mut nonce: U256 = [0; 32];
                    nonce[..prefix.len()].copy_from_slice(prefix);
                    nonce[prefix.len()..].copy_from_slice(&[bit_pos]);
                    nonce
                })
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use bnum::BUintD8;
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[test]
    fn test() {
        type N = BUintD8<32>;

        let mut m = BitMap256::<HashMap<U248, U256>>::default();

        for n in [N::ZERO, N::ONE, N::MAX - N::ONE, N::MAX].map(Into::into) {
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
    #[case(&[])]
    #[case(&[hex!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")])]
    #[case(&[hex!("0000000000000000000000000000000000000000000000000000000000000000"), hex!("0000000000000000000000000000000000000000000000000000000000000001")])]
    #[case(&[hex!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff00"), hex!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")])]
    #[case(&[hex!("0000000000000000000000000000000000000000000000000000000000000000"), hex!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")])]
    fn iter(#[case] nonces: &[U256]) {
        let mut m = BitMap256::<HashMap<U248, U256>>::default();
        for n in nonces {
            assert!(!m.set_bit(*n));
        }

        let all: HashSet<_> = m.as_iter().collect();
        assert_eq!(all, nonces.iter().copied().collect());
    }
}

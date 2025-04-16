use defuse_bitmap::{BitMap256, U248, U256};
use defuse_map_utils::Map;
use near_sdk::near;

pub type Nonce = U256;

/// See [permit2 nonce schema](https://docs.uniswap.org/contracts/permit2/reference/signature-transfer#nonce-schema)
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, Default)]
pub struct Nonces<T: Map<K = U248, V = U256>>(BitMap256<T>);

impl<T> Nonces<T>
where
    T: Map<K = U248, V = U256>,
{
    #[must_use]
    #[inline]
    pub const fn new(bitmap: T) -> Self {
        Self(BitMap256::new(bitmap))
    }

    #[must_use]
    #[inline]
    pub fn is_used(&self, n: Nonce) -> bool {
        self.0.get_bit(n)
    }

    #[must_use]
    #[inline]
    pub fn commit(&mut self, n: Nonce) -> bool {
        !self.0.set_bit(n)
    }
}

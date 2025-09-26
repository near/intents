mod expirable;
mod salted;
mod verifier;
mod versioned;

pub use expirable::ExpirableNonce;

use defuse_bitmap::{BitMap256, U248, U256};
use defuse_borsh_utils::adapters::{As, TimestampNanoSeconds};
use defuse_map_utils::{IterableMap, Map};
use hex_literal::hex;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    near,
};

use crate::{Deadline, DefuseError, Result};

pub type Nonce = U256;

/// See [permit2 nonce schema](https://docs.uniswap.org/contracts/permit2/reference/signature-transfer#nonce-schema)
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, Default)]
pub struct Nonces<T: Map<K = U248, V = U256>>(BitMap256<T>);

impl<T> Nonces<T>
where
    T: Map<K = U248, V = U256>,
{
    #[inline]
    pub const fn new(bitmap: T) -> Self {
        Self(BitMap256::new(bitmap))
    }

    #[inline]
    pub fn is_used(&self, n: Nonce) -> bool {
        self.0.get_bit(n)
    }

    #[inline]
    pub fn commit(&mut self, n: Nonce) -> Result<()> {
        // if ExpirableNonce::maybe_from(n).is_some_and(|expirable| expirable.has_expired()) {
        //     return Err(DefuseError::NonceExpired);
        // }

        if self.0.set_bit(n) {
            return Err(DefuseError::NonceUsed);
        }

        Ok(())
    }

    #[inline]
    pub fn clear_expired(&mut self, n: Nonce) -> bool {
        // if ExpirableNonce::maybe_from(n).is_some_and(|n| n.has_expired()) {
        //     let [prefix @ .., _] = n;
        //     return self.0.clear_by_prefix(prefix);
        // }

        false
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = Nonce> + '_
    where
        T: IterableMap,
    {
        self.0.as_iter()
    }
}

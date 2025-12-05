mod ops;
mod str;

pub use self::str::*;

use defuse_num_utils::CheckedMulDiv;
use near_sdk::borsh::{BorshDeserialize, BorshSerialize, io};
use serde_with::{DeserializeFromStr, SerializeDisplay};

/// Floating point unsigned decimal price, i.e. dst per 1 src
/// always reduced (i.e. normalized)
/// TODO: docs
#[derive(near_sdk::NearSchema)]
#[abi(json, borsh)]
#[schemars(with = "String")]
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, BorshSerialize, SerializeDisplay, DeserializeFromStr,
)]
#[borsh(crate = "::near_sdk::borsh")]
pub struct Price(u8, u128);

impl Price {
    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
    const MAX_DECIMALS: u8 = u128::MAX.ilog10() as u8;

    pub const MIN: Self = Self(Self::MAX_DECIMALS, 1);
    pub const MAX: Self = Self(0, u128::MAX);

    pub const ZERO: Self = Self(0, 0);
    pub const ONE: Self = Self(0, 1);

    #[inline]
    pub const fn new(mut decimals: u8, mut digits: u128) -> Option<Self> {
        if digits == 0 {
            return Some(Self::ZERO);
        }

        while decimals > 0 && digits % 10 == 0 {
            digits /= 10;
            decimals -= 1;
        }

        if decimals > Self::MAX_DECIMALS {
            return None;
        }

        Some(Self(decimals, digits))
    }

    #[inline]
    pub const fn decimals(&self) -> u8 {
        self.0
    }

    #[inline]
    pub const fn digits(&self) -> u128 {
        self.1
    }

    #[inline]
    fn denominator(self) -> u128 {
        // this is safe since decimals are always <= Self::MAX_DECIMALS
        10u128.pow(self.decimals().into())
    }

    pub const fn is_zero(&self) -> bool {
        self.digits() == 0
    }

    #[inline]
    pub fn dst_ceil_checked(self, src_amount: u128) -> Option<u128> {
        src_amount.checked_mul_div_ceil(self.digits(), self.denominator())
    }

    #[inline]
    pub fn dst_floor_checked(self, src_amount: u128) -> Option<u128> {
        src_amount.checked_mul_div(self.digits(), self.denominator())
    }

    #[inline]
    pub fn src_ceil_checked(self, dst_amount: u128) -> Option<u128> {
        dst_amount.checked_mul_div_ceil(self.denominator(), self.digits())
    }

    #[inline]
    pub fn src_floor_checked(self, dst_amount: u128) -> Option<u128> {
        dst_amount.checked_mul_div(self.denominator(), self.digits())
    }
}

impl From<u128> for Price {
    #[inline]
    fn from(value: u128) -> Self {
        Self(0, value)
    }
}

impl BorshDeserialize for Price {
    fn deserialize_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let (decimals, digits) = BorshDeserialize::deserialize_reader(reader)?;
        Self::new(decimals, digits).ok_or_else(|| io::ErrorKind::InvalidData.into())
    }
}

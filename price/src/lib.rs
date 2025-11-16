mod str;

pub use self::str::*;

use defuse_num_utils::CheckedMulDiv;
use near_sdk::near;
use serde_with::{DeserializeFromStr, SerializeDisplay};

/// Floating point unsigned decimal price, i.e. src per 1 dst
/// always reduced (i.e. normalized)
/// TODO: docs
#[near(serializers = [borsh])]
#[cfg_attr(
    feature = "abi",
    derive(near_sdk::NearSchema),
    schemars(with = "String")
)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, SerializeDisplay, DeserializeFromStr)]
pub struct Price(u8, u128);

impl Price {
    const BASE: u128 = 10u128;
    const MAX_DECIMALS: u8 = u128::MAX.ilog(Self::BASE) as u8;

    pub const MIN: Self = Self(Self::MAX_DECIMALS, 1);
    pub const MAX: Self = Self(0, u128::MAX);

    pub const ZERO: Self = Self(0, 0);
    pub const ONE: Self = Self(0, 1);

    #[inline]
    pub const fn new(mut decimals: u8, mut digits: u128) -> Option<Self> {
        if digits == 0 {
            return Some(Self::ZERO);
        }

        while decimals > 0 && digits % Self::BASE == 0 {
            digits /= Self::BASE;
            decimals -= 1;
        }

        if decimals > Self::MAX_DECIMALS {
            return None;
        }

        Some(Self(decimals, digits))
    }

    #[inline]
    pub const fn decimals(self) -> u8 {
        self.0
    }

    #[inline]
    pub const fn digits(self) -> u128 {
        self.1
    }

    #[inline]
    const fn denominator(self) -> u128 {
        Self::BASE
            // this is safe since decimals are always <= Self::MAX_DECIMALS
            .pow(self.decimals() as u32)
    }

    #[inline]
    pub fn dst_ceil(self, src_amount: u128) -> Option<u128> {
        src_amount.checked_mul_div_ceil(self.denominator(), self.digits())
    }

    #[inline]
    pub fn dst_floor(self, src_amount: u128) -> Option<u128> {
        src_amount.checked_mul_div(self.denominator(), self.digits())
    }

    #[inline]
    pub fn src_ceil(self, dst_amount: u128) -> Option<u128> {
        dst_amount.checked_mul_div_ceil(self.digits(), self.denominator())
    }

    #[inline]
    pub fn src_floor(self, dst_amount: u128) -> Option<u128> {
        dst_amount.checked_mul_div(self.digits(), self.denominator())
    }
}

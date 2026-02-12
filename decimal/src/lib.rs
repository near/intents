#[cfg(feature = "arbitrary")]
mod arbitrary;
mod ops;
mod str;

pub use self::str::*;

use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize, io},
    serde_with::{DeserializeFromStr, SerializeDisplay},
};

/// Floating point unsigned decimal price, i.e. dst per 1 src
/// always reduced (i.e. normalized)
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    derive(near_sdk::NearSchema),
    abi(json, borsh),
    schemars(with = "String")
)]
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, BorshSerialize, SerializeDisplay, DeserializeFromStr,
)]
#[borsh(crate = "::near_sdk::borsh")]
#[serde_with(crate = "::near_sdk::serde_with")]
pub struct UD128(u8, u128);

impl UD128 {
    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
    pub(crate) const MAX_DECIMALS: u8 = u128::MAX.ilog10() as u8;

    pub const MIN: Self = Self(Self::MAX_DECIMALS, 1);
    pub const MAX: Self = Self(0, u128::MAX);

    pub const ZERO: Self = Self(0, 0);
    pub const ONE: Self = Self(0, 1);

    #[inline]
    pub const fn new(mut decimals: u8, mut digits: u128) -> Option<Self> {
        if digits == 0 {
            return Some(Self::ZERO);
        }

        // normalize
        {
            macro_rules! strip {
                ($shift:expr) => {{
                    const FACTOR: u128 = 10u128.pow($shift);
                    while decimals >= $shift && digits % FACTOR == 0 {
                        digits /= FACTOR;
                        decimals -= $shift;
                    }
                }};
            }
            strip!(16);
            strip!(8);
            strip!(4);
            strip!(2);
            strip!(1);
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
    pub(crate) const fn denominator(self) -> u128 {
        // this is safe since decimals are always <= Self::MAX_DECIMALS
        #[allow(clippy::as_conversions)]
        10u128.pow(self.decimals() as u32)
    }

    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.digits() == 0
    }
}

impl From<u128> for UD128 {
    #[inline]
    fn from(value: u128) -> Self {
        Self(0, value)
    }
}

impl BorshDeserialize for UD128 {
    fn deserialize_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let (decimals, digits) = BorshDeserialize::deserialize_reader(reader)?;
        Self::new(decimals, digits).ok_or_else(|| io::ErrorKind::InvalidData.into())
    }
}

use core::{
    fmt::{self, Display},
    ops::{Add, Div, Mul, Not, Sub},
};
use std::borrow::Cow;

use defuse_num_utils::{CheckedAdd, CheckedMulDiv, CheckedSub};
use near_sdk::{
    AccountId, AccountIdRef,
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    near,
};
use thiserror::Error as ThisError;

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
pub struct FeesConfig {
    pub fee: Pips,
    pub fee_collector: AccountId,
}

/// 1 pip == 1/100th of bip == 0.0001%
#[near(serializers = [json])]
#[serde(try_from = "u32")]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, BorshSerialize, BorshSchema,
)]
#[borsh(crate = "::near_sdk::borsh")]
pub struct Pips(u32);

impl Pips {
    pub const ZERO: Self = Self(0);
    pub const ONE_PIP: Self = Self(1);
    pub const ONE_BIP: Self = Self(Self::ONE_PIP.as_pips() * 100);
    pub const ONE_PERCENT: Self = Self(Self::ONE_BIP.as_pips() * 100);
    pub const MAX: Self = Self(Self::ONE_PERCENT.as_pips() * 100);

    #[inline]
    pub const fn from_pips(pips: u32) -> Option<Self> {
        if pips > Self::MAX.as_pips() {
            return None;
        }
        Some(Self(pips))
    }

    #[inline]
    pub const fn from_bips(bips: u32) -> Option<Self> {
        Self::ONE_BIP.checked_mul(bips)
    }

    #[inline]
    pub const fn from_percent(percent: u32) -> Option<Self> {
        Self::ONE_PERCENT.checked_mul(percent)
    }

    #[inline]
    pub const fn as_pips(self) -> u32 {
        self.0
    }

    #[inline]
    pub const fn as_bips(self) -> u32 {
        self.as_pips() / Self::ONE_BIP.as_pips()
    }

    #[inline]
    pub const fn as_percent(self) -> u32 {
        self.as_pips() / Self::ONE_PERCENT.as_pips()
    }

    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub fn as_f64(self) -> f64 {
        f64::from(self.as_pips()) / f64::from(Self::MAX.as_pips())
    }

    #[inline]
    pub const fn checked_mul(self, rhs: u32) -> Option<Self> {
        let Some(pips) = self.as_pips().checked_mul(rhs) else {
            return None;
        };
        Self::from_pips(pips)
    }

    #[inline]
    pub const fn checked_div(self, rhs: u32) -> Option<Self> {
        let Some(pips) = self.as_pips().checked_div(rhs) else {
            return None;
        };
        Some(Self(pips))
    }

    #[must_use]
    #[inline]
    pub const fn invert(self) -> Self {
        Self(Self::MAX.as_pips() - self.as_pips())
    }

    #[inline]
    pub fn fee(self, amount: u128) -> u128 {
        amount
            .checked_mul_div(self.as_pips().into(), Self::MAX.as_pips().into())
            .unwrap_or_else(|| unreachable!())
    }

    #[inline]
    pub fn fee_ceil(self, amount: u128) -> u128 {
        amount
            .checked_mul_div_ceil(self.as_pips().into(), Self::MAX.as_pips().into())
            .unwrap_or_else(|| unreachable!())
    }
}

impl CheckedAdd for Pips {
    #[inline]
    fn checked_add(self, rhs: Self) -> Option<Self> {
        self.as_pips()
            .checked_add(rhs.as_pips())
            .and_then(Self::from_pips)
    }
}

impl Add for Pips {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs).unwrap()
    }
}

impl CheckedSub for Pips {
    #[inline]
    fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.as_pips()
            .checked_sub(rhs.as_pips())
            .and_then(Self::from_pips)
    }
}

impl Sub for Pips {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs).unwrap()
    }
}

impl Mul<u32> for Pips {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: u32) -> Self::Output {
        self.checked_mul(rhs).unwrap()
    }
}

impl Div<u32> for Pips {
    type Output = Self;

    #[inline]
    fn div(self, rhs: u32) -> Self::Output {
        self.checked_div(rhs).unwrap()
    }
}

impl Not for Pips {
    type Output = Self;

    fn not(self) -> Self::Output {
        self.invert()
    }
}

impl Display for Pips {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.4}%", self.as_f64() * 100f64)
    }
}

impl TryFrom<u32> for Pips {
    type Error = PipsOutOfRange;

    #[inline]
    fn try_from(pips: u32) -> Result<Self, Self::Error> {
        Self::from_pips(pips).ok_or(PipsOutOfRange)
    }
}

#[derive(Debug, ThisError)]
#[error("out of range: 0..={}", Pips::MAX.as_pips())]
pub struct PipsOutOfRange;

#[must_use = "make sure to `.emit()` this event"]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct FeeChangedEvent {
    pub old_fee: Pips,
    pub new_fee: Pips,
}

#[must_use = "make sure to `.emit()` this event"]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct FeeCollectorChangedEvent<'a> {
    pub old_fee_collector: Cow<'a, AccountIdRef>,
    pub new_fee_collector: Cow<'a, AccountIdRef>,
}

impl BorshDeserialize for Pips {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let pips: u32 = near_sdk::borsh::BorshDeserialize::deserialize_reader(reader)?;
        Self::from_pips(pips).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("{PipsOutOfRange} - Invalid pips value: {pips}"),
            )
        })
    }
}

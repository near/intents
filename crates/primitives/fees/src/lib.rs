use core::{
    fmt::{self, Display},
    ops::{Add, Div, Mul, Not, Sub},
};

use defuse_num_utils::{CheckedAdd, CheckedMulDiv, CheckedSub};

/// 1 pip == 1/100th of bip == 0.0001%
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema)),
    serde(try_from = "u32")
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
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

    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        let Some(pips) = self.as_pips().checked_add(rhs.as_pips()) else {
            return None;
        };
        Self::from_pips(pips)
    }

    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        let Some(pips) = self.as_pips().checked_sub(rhs.as_pips()) else {
            return None;
        };
        Self::from_pips(pips)
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
        self.checked_add(rhs)
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
        self.checked_sub(rhs)
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

#[derive(Debug, thiserror::Error)]
#[error("out of range: 0..={}", Pips::MAX.as_pips())]
pub struct PipsOutOfRange;

#[cfg(feature = "borsh")]
impl ::borsh::BorshDeserialize for Pips {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let pips = u32::deserialize_reader(reader)?;
        Self::from_pips(pips).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("pips: {pips} is {PipsOutOfRange}"),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_constants() {
        assert_eq!(Pips::ONE_BIP.as_pips(), 100);
        assert_eq!(Pips::ONE_PERCENT.as_pips(), 10_000);
        assert_eq!(Pips::MAX.as_pips(), 1_000_000);
        assert_eq!(Pips::MAX.as_percent(), 100);
        assert_eq!(Pips::ONE_PERCENT.as_bips(), 100);
    }

    #[test]
    fn constructors_reject_values_above_max() {
        assert_eq!(Pips::from_pips(1_000_000), Some(Pips::MAX));
        assert_eq!(Pips::from_pips(1_000_001), None);
        assert_eq!(Pips::from_bips(10_000), Some(Pips::MAX));
        assert_eq!(Pips::from_bips(10_001), None);
        assert_eq!(Pips::from_percent(100), Some(Pips::MAX));
        assert_eq!(Pips::from_percent(101), None);
        // u32 overflow in the multiplication is reported as None, not a panic
        assert_eq!(Pips::from_percent(u32::MAX), None);
        assert!(Pips::try_from(1_000_001).is_err());
    }

    #[test]
    fn checked_arithmetic_stays_in_range() {
        let half = Pips::from_percent(50).unwrap();
        assert_eq!(half.checked_add(half), Some(Pips::MAX));
        assert_eq!(Pips::MAX.checked_add(Pips::ONE_PIP), None);
        assert_eq!(Pips::ZERO.checked_sub(Pips::ONE_PIP), None);
        assert_eq!(Pips::MAX.checked_sub(half), Some(half));
        assert_eq!(half.checked_mul(2), Some(Pips::MAX));
        assert_eq!(half.checked_mul(3), None);
        assert_eq!(Pips::MAX.checked_div(2), Some(half));
        assert_eq!(Pips::MAX.checked_div(0), None);
    }

    #[test]
    fn invert_is_complement_to_max() {
        let fee = Pips::from_bips(30).unwrap();
        assert_eq!(fee.invert().as_pips(), 1_000_000 - 3_000);
        assert_eq!(!Pips::ZERO, Pips::MAX);
        assert_eq!(!!fee, fee);
    }

    #[test]
    fn fee_rounds_down_and_fee_ceil_rounds_up() {
        let one_bip = Pips::ONE_BIP;
        assert_eq!(one_bip.fee(10_000), 1);
        assert_eq!(one_bip.fee_ceil(10_000), 1);
        assert_eq!(one_bip.fee(9_999), 0);
        assert_eq!(one_bip.fee_ceil(9_999), 1);
        assert_eq!(one_bip.fee(0), 0);
        assert_eq!(one_bip.fee_ceil(0), 0);
        assert_eq!(Pips::ZERO.fee_ceil(u128::MAX), 0);
    }

    #[test]
    fn fee_never_exceeds_amount_even_at_u128_max() {
        // The intermediate product is computed in 256 bits, so the largest
        // amount at the largest rate does not overflow.
        assert_eq!(Pips::MAX.fee(u128::MAX), u128::MAX);
        assert_eq!(Pips::MAX.fee_ceil(u128::MAX), u128::MAX);
        let near_max = Pips::from_pips(999_999).unwrap();
        assert!(near_max.fee(u128::MAX) < u128::MAX);
    }

    #[test]
    fn display_formats_as_percent() {
        assert_eq!(Pips::MAX.to_string(), "100.0000%");
        assert_eq!(Pips::ONE_PIP.to_string(), "0.0001%");
        assert_eq!(Pips::from_bips(30).unwrap().to_string(), "0.3000%");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_rejects_out_of_range() {
        assert_eq!(serde_json::from_str::<Pips>("1000000").unwrap(), Pips::MAX);
        assert!(serde_json::from_str::<Pips>("1000001").is_err());
    }

    #[cfg(feature = "borsh")]
    #[test]
    fn borsh_rejects_out_of_range() {
        let ok = borsh::to_vec(&1_000_000u32).unwrap();
        assert_eq!(borsh::from_slice::<Pips>(&ok).unwrap(), Pips::MAX);
        let too_big = borsh::to_vec(&1_000_001u32).unwrap();
        assert!(borsh::from_slice::<Pips>(&too_big).is_err());
    }
}

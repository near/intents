use core::{
    cmp::Ordering,
    ops::{Div, Mul},
};

use defuse_num_utils::{CheckedDiv, CheckedMul, CheckedMulDiv};

use crate::UD128;

impl Ord for UD128 {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        let [sd, od]: [u32; 2] = [self, other].map(Self::decimals).map(Into::into);
        let [sm, om] = [self, other].map(Self::digits);

        match sd.cmp(&od) {
            Ordering::Equal => sm.cmp(&om),
            Ordering::Less => sm
                .checked_mul(10u128.pow(od - sd))
                .map_or(Ordering::Greater, |sr| sr.cmp(&om)),
            Ordering::Greater => om
                .checked_mul(10u128.pow(sd - od))
                .map_or(Ordering::Less, |or| sm.cmp(&or)),
        }
    }
}

impl PartialOrd for UD128 {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl CheckedMul<UD128> for u128 {
    #[inline]
    fn checked_mul(self, rhs: UD128) -> Option<Self> {
        self.checked_mul_div(rhs.digits(), rhs.denominator())
    }

    #[inline]
    fn checked_mul_ceil(self, rhs: UD128) -> Option<Self> {
        self.checked_mul_div_ceil(rhs.digits(), rhs.denominator())
    }
}

impl Mul<UD128> for u128 {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: UD128) -> Self::Output {
        CheckedMul::checked_mul(self, rhs).unwrap()
    }
}

impl CheckedDiv<UD128> for u128 {
    #[inline]
    fn checked_div(self, rhs: UD128) -> Option<Self> {
        self.checked_mul_div(rhs.denominator(), rhs.digits())
    }

    #[inline]
    fn checked_div_ceil(self, rhs: UD128) -> Option<Self> {
        self.checked_mul_div_ceil(rhs.denominator(), rhs.digits())
    }
}

impl Div<UD128> for u128 {
    type Output = Self;

    #[inline]
    fn div(self, rhs: UD128) -> Self::Output {
        CheckedDiv::checked_div(self, rhs).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::{Ordering::*, *};

    use rstest::rstest;

    #[rstest]
    #[case("0", Equal, "0")]
    #[case("1", Equal, "1")]
    #[case("123", Equal, "123")]
    #[case("0.1", Equal, "0.1")]
    #[case("0.123", Equal, "0.123")]
    #[case("0.0123", Equal, "0.0123")]
    #[case("1", Greater, "0")]
    #[case("1", Greater, "0.1")]
    #[case("1.1", Greater, "0.1")]
    #[case("1.1", Greater, "1")]
    #[case("123", Greater, "1.1")]
    #[case("340282366920938463463374607431768211455", Greater, "0.1")]
    #[case(
        "340282366920938463463374607431768211455",
        Greater,
        "34028236692093846346337460743176821145.5"
    )]
    fn cmp(#[case] a: &str, #[case] ord: Ordering, #[case] b: &str) {
        let a: UD128 = a.parse().unwrap();
        let b: UD128 = b.parse().unwrap();

        assert_eq!(a.cmp(&b), ord);
        assert_eq!(b.cmp(&a), ord.reverse(), "reverse ordering mismatch");
    }
}

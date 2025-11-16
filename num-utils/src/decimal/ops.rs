use core::{
    cmp::Ordering,
    ops::{Mul, Neg},
};

use num_traits::CheckedMul;

use super::D120;

impl PartialEq for D120 {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reduce().0 == other.reduce().0
    }
}

impl PartialOrd for D120 {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.is_zero() && other.is_zero() {
            return Some(Ordering::Equal);
        }

        Some(match self.trunc().cmp(&other.trunc()) {
            Ordering::Equal => {
                let sd = self.decimals();
                let od = other.decimals();
                let (sf, of) = match sd.cmp(&od) {
                    Ordering::Equal => (self.fract(), other.fract()),
                    Ordering::Less => (
                        self.fract() * Self::BASE.pow((od - sd) as u32),
                        other.fract(),
                    ),
                    Ordering::Greater => (
                        self.fract(),
                        other.fract() * Self::BASE.pow((sd - od) as u32),
                    ),
                };
                sf.cmp(&of)
            }
            o @ (Ordering::Less | Ordering::Greater) => o,
        })
    }
}

impl Ord for D120 {
    #[track_caller]
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl Neg for D120 {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        self.neg()
    }
}

impl Mul for D120 {
    type Output = Self;

    #[track_caller]
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        self.checked_mul(&rhs).expect("overflow")
    }
}

impl CheckedMul for D120 {
    fn checked_mul(&self, v: &Self) -> Option<Self> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(("0", Ordering::Equal, "0"))]
    #[case(("0", Ordering::Less, "1"))]
    #[case(("1", Ordering::Greater, "0"))]
    #[case(("0.1", Ordering::Greater, "0.01"))]
    fn ord(#[case] (a, ord, b): (&str, Ordering, &str)) {
        let [a, b]: [D120; 2] = [a, b].map(|a| a.parse().unwrap());
        assert_eq!(a.cmp(&b), ord);
    }
}

use core::cmp::Ordering;

use crate::Price;

impl Ord for Price {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        let (sd, sm) = (self.decimals(), self.digits());
        let (od, om) = (other.decimals(), other.digits());

        let (sr, or) = match sd.cmp(&od) {
            Ordering::Equal => return sm.cmp(&om),
            Ordering::Less => {
                let Some(sr) = sm.checked_mul(Self::BASE.pow((od - sd) as u32)) else {
                    return Ordering::Greater;
                };
                (sr, om)
            }
            Ordering::Greater => {
                let Some(or) = om.checked_mul(Self::BASE.pow((sd - od) as u32)) else {
                    return Ordering::Less;
                };
                (sm, or)
            }
        };

        sr.cmp(&or)
    }
}

impl PartialOrd for Price {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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
        let a: Price = a.parse().unwrap();
        let b: Price = b.parse().unwrap();

        assert_eq!(a.cmp(&b), ord);
        assert_eq!(b.cmp(&a), ord.reverse(), "reverse ordering mismatch");
    }
}

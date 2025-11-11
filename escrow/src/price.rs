use defuse_num_utils::CheckedMulDiv;
use near_sdk::near;
use serde_with::{DisplayFromStr, serde_as};

/// Maker / Taker
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
// TODO: deserialize not zero?
// TODO: store as (scale, mantissa)
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price(#[serde_as(as = "DisplayFromStr")] u128);

impl Price {
    // TODO: check for most decimals? check wrap.near?
    const DECIMALS: u32 = 18;

    // pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(10u128.pow(Self::DECIMALS));

    pub fn ratio(src_amount: u128, dst_amount: u128) -> Option<Self> {
        if src_amount == 0 {
            return None;
        }
        src_amount
            // TODO: ceil?
            .checked_mul_div(Self::ONE.0, dst_amount)
            .map(Self)
    }

    pub fn dst_amount(&self, src_amount: u128) -> Option<u128> {
        src_amount.checked_mul_div(Self::ONE.0, self.0)
    }

    pub fn src_amount(&self, dst_amount: u128) -> Option<u128> {
        // TODO: ceil?
        dst_amount.checked_mul_div(self.0, Self::ONE.0)
    }

    pub fn to_f64(&self) -> f64 {
        self.0 as f64 / Self::ONE.0 as f64
    }
}

// TODO: ops

// TODO: more tests
#[cfg(test)]
mod tests {
    use near_sdk::NearToken;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(100, 200)]
    // TODO
    // #[case(NearToken::from_near(1).as_yoctonear(), 10u128.pow(8)/100_000)]
    fn ratio(#[case] src_amount: u128, #[case] dst_amount: u128) {
        let p = Price::ratio(src_amount, dst_amount).unwrap();
        assert_eq!(p.dst_amount(src_amount), Some(dst_amount));
    }

    #[rstest]
    #[case(0, 100)]
    #[case(100, 0)]
    fn zero(#[case] src_amount: u128, #[case] dst_amount: u128) {
        assert_eq!(Price::ratio(src_amount, dst_amount), None);
    }
}

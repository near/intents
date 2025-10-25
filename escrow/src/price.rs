use defuse_num_utils::CheckedMulDiv;
use near_sdk::near;

/// Maker / Taker
#[near(serializers = [borsh, json])]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price(u128);

impl Price {
    pub const ZERO: Self = Self(0);
    // TODO: check for most decimals?
    pub const ONE: Self = Self(10u128.pow(9));

    pub fn dst_amount(&self, src_amount: u128) -> Option<u128> {
        src_amount.checked_mul_div(Self::ONE.0, self.0)
    }

    pub fn src_amount(&self, dst_amount: u128) -> Option<u128> {
        dst_amount.checked_mul_div(self.0, Self::ONE.0)
    }

    pub fn to_f64(&self) -> f64 {
        self.0 as f64 / Self::ONE.0 as f64
    }
}

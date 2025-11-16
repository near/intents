use defuse_num_utils::CheckedMulDiv;
use fastnum::{UD128, decimal::Context, int::UInt};
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize, io},
    near,
};
use num_traits::{AsPrimitive, ToPrimitive};
use serde_with::{DisplayFromStr, serde_as};

/// Src per 1 dst
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
#[repr(transparent)]
pub struct Price(
    #[borsh(
        serialize_with = "serialize",
        deserialize_with = "deserialize",
        schema(with_funcs(
            declaration = "u128::declaration",
            definitions = "u128::add_definitions_recursively",
        ))
    )]
    #[cfg_attr(
        all(feature = "abi", not(target_arch = "wasm32")),
        schemars(with = "String")
    )]
    UD128,
);

impl Price {
    // TODO: check for most decimals? check wrap.near?
    // const DECIMALS: u32 = 18;

    // pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(UD128::ONE);

    pub fn from_parts(digits: u128, scale: i32) -> Self {
        Self(UD128::from_parts(digits.as_(), scale, Context::default()))
    }

    pub fn dst_amount(&self, src_amount: u128) -> Option<u128> {
        src_amount.checked_mul_div(self.denominator(), self.nominator())
    }

    pub fn src_amount(&self, dst_amount: u128) -> Option<u128> {
        // TODO: ceil?
        dst_amount.checked_mul_div(self.nominator(), self.denominator())
    }

    fn nominator(&self) -> u128 {
        self.0.digits().as_()
    }

    fn scale(&self) -> u32 {
        self.0.fractional_digits_count().try_into().unwrap()
    }

    fn denominator(&self) -> u128 {
        10u128.pow(self.scale())
    }

    // pub fn to_f64(&self) -> f64 {
    //     self.0 as f64 / Self::ONE.0 as f64
    // }
}

// TODO
fn serialize(value: &UD128, w: &mut impl io::Write) -> io::Result<()> {
    let value = value.reduce();
    (
        value.fractional_digits_count(),
        value.digits().to_be().digits(),
    )
        .serialize(w)
}

fn deserialize(r: &mut impl io::Read) -> io::Result<UD128> {
    let (fractional_digits_count, digits) = <(i16, [u64; 2])>::deserialize_reader(r)?;
    Ok(UD128::from_parts(
        UInt::from_digits(digits),
        fractional_digits_count as i32,
        Context::default(),
    )
    .reduce())
}

// TODO: ops

// TODO: more tests
// #[cfg(test)]
// mod tests {
//     use rstest::rstest;

//     use super::*;

//     #[rstest]
//     #[case(100, 200)]
//     // TODO
//     // #[case(NearToken::from_near(1).as_yoctonear(), 10u128.pow(8)/100_000)]
//     fn ratio(#[case] src_amount: u128, #[case] dst_amount: u128) {
//         let p = Price::ratio(src_amount, dst_amount).unwrap();
//         assert_eq!(p.dst_amount(src_amount), Some(dst_amount));
//     }

//     #[rstest]
//     #[case(0, 100)]
//     #[case(100, 0)]
//     fn zero(#[case] src_amount: u128, #[case] dst_amount: u128) {
//         assert_eq!(Price::ratio(src_amount, dst_amount), None);
//     }
// }
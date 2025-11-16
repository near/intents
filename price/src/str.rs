use core::{
    fmt::{self, Debug, Display},
    num::ParseIntError,
    str::FromStr,
};

use thiserror::Error as ThisError;

use crate::Price;

impl Debug for Price {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for Price {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = self.digits();
        let denominator = self.denominator();
        let integer = digits / denominator;
        write!(f, "{integer}")?;

        let fract = digits % denominator;
        if fract != 0 {
            let decimals = self.decimals() as usize;
            write!(f, ".{:0decimals$}", fract)?;
        }
        Ok(())
    }
}

impl FromStr for Price {
    type Err = ParseDecimalError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (integer, fract) = s.split_once('.').map_or((s, None), |(i, f)| (i, Some(f)));

        if integer.starts_with('+') {
            return Err(ParseDecimalError::InvalidFormat);
        }

        let mut digits = if integer.is_empty() {
            if fract.is_none_or(str::is_empty) {
                return Err(ParseDecimalError::InvalidFormat);
            }
            0
        } else {
            u128::from_str_radix(integer, Self::BASE as u32)?
        };

        let decimals = if let Some(fract) = fract
            .map(|s| s.trim_end_matches('0'))
            .filter(|s| !s.is_empty())
        {
            if fract.starts_with('+') {
                return Err(ParseDecimalError::InvalidFormat);
            }

            let decimals: u8 = fract
                .len()
                .try_into()
                .ok()
                .filter(|d| *d <= Self::MAX_DECIMALS)
                .ok_or(ParseDecimalError::Overflow)?;

            let fract = u128::from_str_radix(fract, Self::BASE as u32)?;
            digits = digits
                .checked_mul(Self::BASE.pow(decimals as u32))
                .and_then(|d| d.checked_add(fract))
                .ok_or(ParseDecimalError::Overflow)?;

            decimals
        } else {
            0
        };

        Self::new(decimals, digits).ok_or(ParseDecimalError::Overflow)
    }
}

#[derive(Debug, ThisError)]
pub enum ParseDecimalError {
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
    #[error("invalid format")]
    InvalidFormat,
    #[error("overflow")]
    Overflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case("0", "0")]
    #[case("0000", "0")]
    #[case("00.0", "0")]
    #[case("000.0000000", "0")]
    #[case("1", "1")]
    #[case("001", "1")]
    #[case("1.0", "1")]
    #[case("1.0000", "1")]
    #[case("0.1", "0.1")]
    #[case("0.10", "0.1")]
    #[case("0.01", "0.01")]
    #[case("0.01000", "0.01")]
    #[case(".0", "0")]
    #[case(".000", "0")]
    #[case(".1", "0.1")]
    #[case(".10", "0.1")]
    #[case(".01", "0.01")]
    #[case(".01000", "0.01")]
    #[case(".0000000000000000000000000000000000000000000000000", "0")]
    #[case("1.0000000000000000000000000000000000000000000000000", "1")]
    #[case(
        ".00000000000000000000000000000000000001",
        "0.00000000000000000000000000000000000001"
    )]
    #[case(
        "0001.00000000000000000000000000000000000001",
        "1.00000000000000000000000000000000000001"
    )]
    #[case(
        "340282366920938463463374607431768211455",
        "340282366920938463463374607431768211455"
    )]
    #[case(
        "340282366920938463463374607431768211455.",
        "340282366920938463463374607431768211455"
    )]
    #[case(
        "340282366920938463463374607431768211455.000",
        "340282366920938463463374607431768211455"
    )]
    #[case(
        "34028236692093846346337460743176821145.500",
        "34028236692093846346337460743176821145.5"
    )]
    #[case(
        "3.40282366920938463463374607431768211455",
        "3.40282366920938463463374607431768211455"
    )]
    fn roundtrip(#[case] input: &str, #[case] result: &str) {
        let p: Price = input.parse().unwrap();
        assert_eq!(p.to_string(), result)
    }

    #[rstest]
    #[case::empty("")]
    #[case("+")]
    #[case("-")]
    #[case::only_dot(".")]
    #[case("+0")]
    #[case("+0.")]
    #[case("-0")]
    #[case("-0.0")]
    #[case("0+")]
    #[case("0-")]
    #[case("..")]
    #[case("0..")]
    #[case("..0")]
    #[case("0..0")]
    #[case(".0.")]
    #[case("0.0.")]
    #[case(".0.0")]
    #[case("0.0.0")]
    #[case("0.000.0")]
    #[case("0.+0")]
    #[case("0.-0")]
    #[case::integer_overflow("340282366920938463463374607431768211456")]
    #[case::integer_overflow("340282366920938463463374607431768211456.0")]
    #[case::integer_overflow("34028236692093846346337460743176821145.6")]
    #[case::integer_overflow("3.40282366920938463463374607431768211456")]
    #[case::decimals_overflow(".000000000000000000000000000000000000001")]
    fn invalid(#[case] s: &str) {
        s.parse::<Price>().unwrap_err();
    }
}

use core::{
    fmt::{self, Display},
    num::ParseIntError,
    str::FromStr,
};

use thiserror::Error as ThisError;

use crate::decimal::D120;

impl Display for D120 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.trunc())?;

        let decimal_part = self.fract();
        if decimal_part != 0 {
            write!(f, ".{}", decimal_part)?;
        }

        Ok(())
    }
}

impl FromStr for D120 {
    type Err = ParseDecimalError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (is_negative, decimals, digits) = if let Some((integer, decimal)) = s.split_once('.') {
            let i = i128::from_str_radix(integer, Self::BASE as u32)?;
            let d = u128::from_str_radix(decimal, Self::BASE as u32)?;
            let decimals: u8 = decimal
                .len()
                .try_into()
                .map_err(|_| ParseDecimalError::Overflow)?;

            (
                i.is_negative(),
                decimals,
                i.unsigned_abs()
                    .checked_mul(Self::BASE.pow(decimals as u32))
                    .and_then(|v| v.checked_add(d))
                    .ok_or(ParseDecimalError::Overflow)?,
            )
        } else {
            let i = i128::from_str_radix(s, Self::BASE as u32)?;
            (i.is_negative(), 0, i.unsigned_abs())
        };
        Self::new(is_negative, decimals, digits).ok_or(ParseDecimalError::Overflow)
    }
}

#[derive(Debug, ThisError)]
pub enum ParseDecimalError {
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
    #[error("overflow")]
    Overflow,
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("0")]
    #[case("0.1")]
    #[case("0.01")]
    // TODO
    fn roundtrip(#[case] s: &str) {
        let d: D120 = s.parse().unwrap();
        assert_eq!(d.to_string(), s);
    }
}

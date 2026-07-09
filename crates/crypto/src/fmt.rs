use crate::Curve;

pub trait TypedCurve: Curve {
    const CURVE_TYPE: &str;

    #[inline]
    fn to_base58(bytes: impl AsRef<[u8]>) -> String {
        format!(
            "{}:{}",
            Self::CURVE_TYPE,
            bs58::encode(bytes.as_ref()).into_string()
        )
    }

    fn parse_base58<const N: usize>(s: impl AsRef<str>) -> Result<[u8; N], ParseCurveError> {
        let s = s.as_ref();
        let data = if let Some((curve, data)) = s.split_once(':') {
            if !curve.eq_ignore_ascii_case(Self::CURVE_TYPE) {
                return Err(ParseCurveError::WrongCurveType);
            }
            data
        } else {
            s
        };
        checked_base58_decode_array(data)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseCurveError {
    #[error("wrong curve type")]
    WrongCurveType,
    #[error("base58: {0}")]
    Base58(#[from] bs58::decode::Error),
    #[error("invalid length")]
    InvalidLength,
}

pub fn checked_base58_decode_array<const N: usize>(
    input: impl AsRef<[u8]>,
) -> Result<[u8; N], ParseCurveError> {
    let mut output = [0u8; N];
    let n = bs58::decode(input.as_ref())
        // NOTE: `.into_array_const()` doesn't return an error on insufficient
        // input length and pads the array with zeros
        .onto(&mut output)?;
    (n == N)
        .then_some(output)
        .ok_or(ParseCurveError::InvalidLength)
}

use crate::Curve;

/// Helper trait to encode/decode bytes as `<CURVE>:<base58>`
pub trait TypedCurve: Curve {
    /// Used as prefix: `<CURVE>:...`
    const CURVE_TYPE: &str;

    /// Encodes bytes to string as `<CURVE>:<base58>`
    #[inline]
    fn to_base58(bytes: impl AsRef<[u8]>) -> String {
        format!(
            "{}:{}",
            Self::CURVE_TYPE,
            bs58::encode(bytes.as_ref()).into_string()
        )
    }

    /// Decodes bytes from string as `<CURVE>:<base58>`
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

/// An error returned from [`TypedCurve::parse_base58`]
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseCurveError {
    #[error("wrong curve type")]
    WrongCurveType,
    #[error("base58: {0}")]
    Base58(#[from] bs58::decode::Error),
    #[error("invalid length")]
    InvalidLength,
}

/// Base-58 decode bytes array of a an exact size.
/// If decoded length doesn't match `N`, an error will be returned.
///
/// # Examples
///
/// ```rust
/// # use defuse_crypto::fmt::{checked_base58_decode_array, ParseCurveError};
/// # use hex_literal::hex;
/// assert_eq!(
///     checked_base58_decode_array::<8>("he11owor1d")?,
///     hex!("04305e2b2473f058"),
/// );
/// checked_base58_decode_array::<7>("he11owor1d").expect_err("buffer too small");
/// checked_base58_decode_array::<9>("he11owor1d").expect_err("buffer too large");
/// # Ok::<(), ParseCurveError>(())
/// ```
pub fn checked_base58_decode_array<const N: usize>(
    input: impl AsRef<[u8]>,
) -> Result<[u8; N], ParseCurveError> {
    let mut output = [0u8; N];
    let n = bs58::decode(input.as_ref())
        // NOTE: `.into_array_const()` doesn't return an error on insufficient
        // input length and pads the array with zeros
        .onto(&mut output)?;
    if n != N {
        return Err(ParseCurveError::InvalidLength);
    }
    Ok(output)
}

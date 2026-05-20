use thiserror::Error as ThisError;

use crate::Curve;
pub trait TypedCurve: Curve {
    const CURVE_TYPE: CurveType;

    #[inline]
    fn to_base58(bytes: impl AsRef<[u8]>) -> String {
        format!(
            "{}:{}",
            Self::CURVE_TYPE,
            bs58::encode(bytes.as_ref()).into_string()
        )
    }

    fn parse_base58<const N: usize>(s: impl AsRef<str>) -> Result<[u8; N], crate::ParseCurveError> {
        let s = s.as_ref();
        let data = if let Some((curve, data)) = s.split_once(':') {
            if !curve.eq_ignore_ascii_case(Self::CURVE_TYPE.into()) {
                return Err(crate::ParseCurveError::WrongCurveType);
            }
            data
        } else {
            s
        };
        checked_base58_decode_array(data)
    }
}

#[derive(strum::Display, strum::IntoStaticStr, strum::EnumString)]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
#[repr(u8)]
pub enum CurveType {
    #[cfg(feature = "ed25519")]
    Ed25519 = 0,
    #[cfg(feature = "secp256k1")]
    Secp256k1 = 1,
    #[cfg(feature = "p256")]
    P256 = 2,
}

#[derive(Debug, ThisError, PartialEq, Eq)]
pub enum ParseCurveError {
    #[error("wrong curve type")]
    WrongCurveType,
    #[error("base58: {0}")]
    Base58(#[from] bs58::decode::Error),
    #[error("invalid length")]
    InvalidLength,
}

fn checked_base58_decode_array<const N: usize>(
    input: impl AsRef<[u8]>,
) -> Result<[u8; N], crate::ParseCurveError> {
    let mut output = [0u8; N];
    let n = bs58::decode(input.as_ref())
        // NOTE: `.into_array_const()` doesn't return an error on insufficient
        // input length and pads the array with zeros
        .onto(&mut output)?;
    (n == N)
        .then_some(output)
        .ok_or(crate::ParseCurveError::InvalidLength)
}

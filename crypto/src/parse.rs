use near_sdk::bs58;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum ParseCurveError {
    #[error("wrong curve type")]
    WrongCurveType,
    #[error("base58: {0}")]
    Base58(#[from] bs58::decode::Error),
    #[error("invalid length")]
    InvalidLength,
}

/// Decodes input as base58 into array and checks for its length
pub fn checked_base58_decode_array<const N: usize>(
    input: impl AsRef<[u8]>,
) -> Result<[u8; N], ParseCurveError> {
    let mut output = [0u8; N];
    let n = bs58::decode(input.as_ref())
        // NOTE: `.into_array_const()` doesn't return an error on insufficient
        // input lentgh and returns array filled up with zeros
        .onto(&mut output)?;
    if n != N {
        return Err(ParseCurveError::InvalidLength);
    }
    Ok(output)
}

use arbitrary::Arbitrary;
use hex::ToHex;

use crate::arbitrary::array::Array;

/// N is the number of bytes, NOT hex characters
pub fn arbitrary_hex<const N: usize>(
    u: &mut arbitrary::Unstructured<'_>,
) -> arbitrary::Result<String> {
    let data = Array::<u8, N>::arbitrary(u)?;
    let data: [u8; N] = data.into();
    Ok(data.encode_hex())
}

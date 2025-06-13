use arbitrary::Arbitrary;
use hex::ToHex;

/// N is the number of bytes, NOT hex characters
pub fn arbitrary_hex_fixed_size<const N: usize>(
    u: &mut arbitrary::Unstructured<'_>,
) -> arbitrary::Result<String> {
    let data = <[u8; N]>::arbitrary(u)?;
    Ok(data.encode_hex())
}

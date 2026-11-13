//! Hex encoding

use std::io;

use crate::{AsBytes, IoDecoder, IoEncoder};

/// Hex codec.
///
/// # Example
///
/// ```rust
/// use defuse_codec::{IoDecoder, IoEncoder, hex::Hex};
///
/// let value = b"Hello, Near!";
/// let encoded = Hex::to_vec(&value).unwrap();
/// assert_eq!(encoded, b"48656c6c6f2c204e65617221");
///
/// let decoded: Vec<u8> = Hex::from_slice(encoded).unwrap();
/// assert_eq!(decoded, value);
/// ```
// TODO: add support for:
// * Lower/Upper/CheckSum format?
// * optional "0x" prefix and whether to enforce it on decode?
#[non_exhaustive]
pub struct Hex;

impl<T> IoEncoder<T> for Hex
where
    T: AsRef<[u8]> + ?Sized,
{
    #[inline]
    fn to_writer<W>(writer: W, value: &T) -> io::Result<()>
    where
        W: io::Write,
    {
        AsBytes::to_writer(writer, &hex::encode(value))
    }

    #[inline]
    fn to_vec(value: &T) -> io::Result<Vec<u8>> {
        Self::to_string(value).map(Into::into)
    }

    #[inline]
    fn to_string(value: &T) -> io::Result<String> {
        Ok(hex::encode(value))
    }

    #[inline]
    fn encoded_len(value: &T) -> io::Result<usize> {
        value
            .as_ref()
            .len()
            .checked_mul(2)
            .ok_or(io::ErrorKind::OutOfMemory)
            .map_err(Into::into)
    }
}

impl<T> IoDecoder<T> for Hex
where
    T: TryFrom<Vec<u8>>,
{
    #[inline]
    fn from_reader<R>(mut reader: R) -> io::Result<T>
    where
        R: io::Read,
    {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        Self::from_slice(buf)
    }

    #[inline]
    fn from_slice(bytes: impl AsRef<[u8]>) -> io::Result<T> {
        // TODO: use `const-hex` or `faster-hex` crates instead?
        let buf =
            hex::decode(bytes).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        AsBytes::try_from_vec(buf)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::test_utils::assert_roundtrip;

    use super::*;

    #[rstest]
    #[case::empty(b"")]
    #[case::dot(b".")]
    #[case::test(b"test")]
    #[case::hello(b"Hello, Near!")]
    #[case::hash(&[42u8; 32])]
    fn roundtrip_array<const N: usize>(#[case] v: &[u8; N]) {
        assert_roundtrip::<Hex, _>(v, None);
    }

    #[rstest]
    #[case::empty(b"")]
    #[case::dot(b".")]
    #[case::test(b"test")]
    #[case::hello(b"Hello, Near!")]
    #[case::hash(&[42u8; 32])]
    fn roundtrip_slice(#[case] v: &[u8]) {
        assert_roundtrip::<Hex, _>(v, None);
    }

    #[rstest]
    #[case::empty("")]
    #[case::dot(".")]
    #[case::test("test")]
    #[case::hello("Hello, Near!")]
    fn roundtrip_str(#[case] v: &str) {
        assert_roundtrip::<Hex, _>(v, None);
    }

    #[rstest]
    #[case::whitespace(b" ")]
    #[case::newline(b"\n")]
    #[case::leading_whitespace(b" 48656c6c6f2c204e65617221")]
    #[case::trailing_whitespace(b"48656c6c6f2c204e65617221 ")]
    fn invalid_input(#[case] bytes: &[u8]) {
        <Hex as IoDecoder<Vec<u8>>>::from_slice(bytes).unwrap_err();
    }
}

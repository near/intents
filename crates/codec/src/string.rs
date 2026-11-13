use std::{fmt::Display, io, str::FromStr};

use crate::{IoDecoder, IoEncoder};

/// A codec that encodes using `Display` implementation and decodes via `FromStr`.
///
/// # Example
///
/// ```rust
/// use defuse_codec::{IoDecoder, IoEncoder, DisplayFromStr};
///
/// let value: u8 = 42;
/// let encoded = DisplayFromStr::to_vec(&value).unwrap();
/// assert_eq!(encoded, b"42");
///
/// let decoded: u8 = DisplayFromStr::from_slice(encoded).unwrap();
/// assert_eq!(decoded, value);
/// ```
#[non_exhaustive]
pub struct DisplayFromStr;

impl<T> IoEncoder<T> for DisplayFromStr
where
    T: Display + ?Sized,
{
    #[inline]
    fn to_writer<W>(mut writer: W, value: &T) -> io::Result<()>
    where
        W: io::Write,
    {
        write!(writer, "{value}")
    }

    #[inline]
    fn to_string(value: &T) -> io::Result<String> {
        Ok(value.to_string())
    }
}

impl<T> IoDecoder<T> for DisplayFromStr
where
    T: FromStr<Err: Display>,
{
    #[inline]
    fn from_reader<R>(mut reader: R) -> io::Result<T>
    where
        R: io::Read,
    {
        let mut buf = String::new();
        reader.read_to_string(&mut buf)?;
        Self::from_str(buf)
    }

    #[inline]
    fn from_slice(bytes: impl AsRef<[u8]>) -> io::Result<T> {
        str::from_utf8(bytes.as_ref())
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
            .and_then(Self::from_str)
    }

    #[inline]
    fn from_str(s: impl AsRef<str>) -> io::Result<T> {
        T::from_str(s.as_ref())
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::test_utils::assert_roundtrip;

    use super::*;

    #[rstest]
    #[case::empty("")]
    #[case::dot(".")]
    #[case::test("test")]
    #[case::hello("Hello, Near!")]
    fn roundtrip_str(#[case] v: &str) {
        assert_roundtrip::<DisplayFromStr, _>(v, None);
    }

    #[rstest]
    #[case::zero(0)]
    #[case::one(1)]
    #[case::value(123)]
    #[case::max(u128::MAX)]
    fn roundtrip_u128(#[case] v: u128) {
        assert_roundtrip::<DisplayFromStr, _>(&v, None);
    }

    #[rstest]
    #[case::whitespace(b" ")]
    #[case::newline(b"\n")]
    #[case::leading_whitespace(b" 123")]
    #[case::trailing_whitespace(b"123 ")]
    fn invalid_input(#[case] bytes: &[u8]) {
        <DisplayFromStr as IoDecoder<u128>>::from_slice(bytes).unwrap_err();
    }
}

//! JSON serialization format.

use std::{io, marker::PhantomData};

use serde_with::{DeserializeAs, Same, SerializeAs, de::DeserializeAsWrap, ser::SerializeAsWrap};

use crate::{IoDecoder, IoEncoder};

/// A codec for [JSON](serde_json) with support for custom [`serde_with`] adaptors.
///
/// # Example
///
/// ```rust
/// use defuse_codec::{IoDecoder, IoEncoder, json::Json};
///
/// let value: Vec<u32> = vec![1, 2, 3];
///
/// let encoded = <Json>::to_vec(&value).unwrap();
/// assert_eq!(encoded, b"[1,2,3]");
/// let decoded: Vec<u32> = <Json>::from_slice(encoded).unwrap();
/// assert_eq!(decoded, value);
///
/// // or via adaptor:
/// use serde_with::DisplayFromStr;
///
/// let as_hex = Json::<Vec<DisplayFromStr>>::to_vec(&value).unwrap();
/// assert_eq!(as_hex, br#"["1","2","3"]"#);
/// let from_hex: Vec<u32> = Json::<Vec<DisplayFromStr>>::from_slice(as_hex).unwrap();
/// assert_eq!(from_hex, value);
/// ```
pub struct Json<As: ?Sized = Same>(PhantomData<As>);

impl<T, As> IoEncoder<T> for Json<As>
where
    T: ?Sized,
    As: SerializeAs<T> + ?Sized,
{
    #[inline]
    fn to_writer<W>(writer: W, value: &T) -> io::Result<()>
    where
        W: io::Write,
    {
        serde_json::to_writer(writer, &SerializeAsWrap::<T, As>::new(value)).map_err(Into::into)
    }

    #[inline]
    fn to_vec(value: &T) -> io::Result<Vec<u8>> {
        serde_json::to_vec(&SerializeAsWrap::<T, As>::new(value)).map_err(Into::into)
    }
}

impl<T, As> IoDecoder<T> for Json<As>
where
    As: for<'a> DeserializeAs<'a, T> + ?Sized,
{
    #[inline]
    fn from_reader<R>(reader: R) -> io::Result<T>
    where
        R: io::Read,
    {
        serde_json::from_reader(reader)
            .map(DeserializeAsWrap::<T, As>::into_inner)
            .map_err(Into::into)
    }

    #[inline]
    fn from_slice(bytes: impl AsRef<[u8]>) -> io::Result<T> {
        serde_json::from_slice(bytes.as_ref())
            .map(DeserializeAsWrap::<T, As>::into_inner)
            .map_err(Into::into)
    }

    #[inline]
    fn from_str(s: impl AsRef<str>) -> io::Result<T> {
        serde_json::from_str(s.as_ref())
            .map(DeserializeAsWrap::<T, As>::into_inner)
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use serde_with::DisplayFromStr;

    use crate::test_utils::assert_roundtrip;

    use super::*;

    #[rstest]
    #[case::empty("")]
    #[case::dot(".")]
    #[case::test("test")]
    #[case::hello("Hello, Near!")]
    fn roundtrip_str(#[case] v: &str) {
        assert_roundtrip::<Json, _>(v);
    }

    #[rstest]
    #[case::zero(0)]
    #[case::one(1)]
    #[case::value(123)]
    #[case::max(u128::MAX)]
    fn roundtrip_u128(#[case] v: u128) {
        assert_roundtrip::<Json<DisplayFromStr>, _>(&v);
    }

    #[rstest]
    #[case::leading(" \n{}")]
    #[case::trailing("{} \n")]
    fn whitespace_ok(#[case] input: &str) {
        <Json as IoDecoder<serde_json::Value>>::from_str(input).unwrap();
    }

    #[rstest]
    #[case::whitespace(b" ")]
    #[case::newline(b"\n")]
    #[case::nul(b"nul")]
    #[case::mixed(b"12.\"three\"")]
    #[case::leading_chars(b"1{}")]
    #[case::leading_chars(b"a{}")]
    #[case::trailing_chars(b"{}1")]
    #[case::trailing_chars(b"{}a")]
    fn invalid_input(#[case] bytes: &[u8]) {
        <Json as IoDecoder<serde_json::Value>>::from_slice(bytes).unwrap_err();
    }
}

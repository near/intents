//! Base64 encoding

use std::{
    io::{self, Read, Write},
    marker::PhantomData,
};

use base64::{
    Engine,
    engine::{
        GeneralPurpose, GeneralPurposeConfig,
        general_purpose::{NO_PAD, PAD},
    },
    read::DecoderReader,
    write::EncoderWriter,
};

use crate::{IoDecoder, IoEncoder};

/// Base64 codec.
///
/// # Example
///
/// ```rust
/// use defuse_codec::{IoDecoder, IoEncoder, base64::Base64};
///
/// let value = vec![0u8; 32];
/// let encoded = <Base64>::to_vec(&value).unwrap();
/// assert_eq!(encoded, b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
///
/// let decoded: Vec<u8> = <Base64>::from_slice(encoded).unwrap();
/// assert_eq!(decoded, value);
/// ```
pub struct Base64<ALPHABET: Alphabet = Standard, PADDING: Padding = Padded>(
    PhantomData<(ALPHABET, PADDING)>,
);

impl<ALPHABET, PADDING> Base64<ALPHABET, PADDING>
where
    ALPHABET: Alphabet,
    PADDING: Padding,
{
    #[inline]
    const fn engine() -> GeneralPurpose {
        GeneralPurpose::new(&ALPHABET::CHARSET, PADDING::CONFIG)
    }
}

impl<T, ALPHABET, PADDING> IoEncoder<T> for Base64<ALPHABET, PADDING>
where
    T: AsRef<[u8]> + ?Sized,
    ALPHABET: Alphabet,
    PADDING: Padding,
{
    #[inline]
    fn to_writer<W>(writer: W, value: &T) -> io::Result<()>
    where
        W: io::Write,
    {
        let engine = Self::engine();
        let mut writer = EncoderWriter::new(writer, &engine);
        writer.write_all(value.as_ref())?;
        writer.finish().map(|_| ())
    }

    #[inline]
    fn to_vec(value: &T) -> io::Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(Self::encoded_len(value)?);
        Self::to_writer(&mut buf, value)?;
        Ok(buf)
    }

    #[inline]
    fn encoded_len(value: &T) -> io::Result<usize> {
        base64::encoded_len(value.as_ref().len(), true)
            .ok_or(io::ErrorKind::OutOfMemory)
            .map_err(Into::into)
    }
}

impl<T, ALPHABET, PADDING> IoDecoder<T> for Base64<ALPHABET, PADDING>
where
    T: TryFrom<Vec<u8>>,
    ALPHABET: Alphabet,
    PADDING: Padding,
{
    #[inline]
    fn from_reader<R>(reader: R) -> io::Result<T>
    where
        R: io::Read,
    {
        let engine = Self::engine();
        // TODO: add tests for non-padded?...
        let mut reader = DecoderReader::new(reader, &engine);

        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;

        let length = buf.len();
        T::try_from(buf).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("can't convert a byte vector of length {length} to the output type"),
            )
        })
    }

    #[inline]
    fn from_slice(bytes: impl AsRef<[u8]>) -> io::Result<T> {
        let buf = Self::engine()
            .decode(bytes)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        let length = buf.len();
        T::try_from(buf).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("can't convert a byte vector of length {length} to the output type"),
            )
        })
    }
}

/// A base64 alphabet
pub trait Alphabet: sealed::Sealed {
    const CHARSET: ::base64::alphabet::Alphabet;
}
/// The standard character set (uses `+` and `/`).
///
/// See [RFC 3548](https://tools.ietf.org/html/rfc3548#section-3).
pub struct Standard;
impl Alphabet for Standard {
    const CHARSET: base64::alphabet::Alphabet = ::base64::alphabet::STANDARD;
}

/// The URL safe character set (uses `-` and `_`).
///
/// See [RFC 3548](https://tools.ietf.org/html/rfc3548#section-3).
pub struct UrlSafe;
impl Alphabet for UrlSafe {
    const CHARSET: base64::alphabet::Alphabet = ::base64::alphabet::URL_SAFE;
}

/// The `crypt(3)` character set (uses `./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz`).
///
/// Not standardized, but folk wisdom on the net asserts that this alphabet is what crypt uses.
pub struct Crypt;
impl Alphabet for Crypt {
    const CHARSET: base64::alphabet::Alphabet = ::base64::alphabet::CRYPT;
}

/// The bcrypt character set (uses `./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789`).
pub struct Bcrypt;
impl Alphabet for Bcrypt {
    const CHARSET: base64::alphabet::Alphabet = ::base64::alphabet::BCRYPT;
}

/// The character set used in IMAP-modified UTF-7 (uses `+` and `,`).
///
/// See [RFC 3501](https://tools.ietf.org/html/rfc3501#section-5.1.3).
pub struct ImapMutf7;
impl Alphabet for ImapMutf7 {
    const CHARSET: base64::alphabet::Alphabet = ::base64::alphabet::IMAP_MUTF7;
}

/// The character set used in `BinHex` 4.0 files.
///
/// See [BinHex 4.0 Definition](http://files.stairways.com/other/binhex-40-specs-info.txt).
pub struct BinHex;
impl Alphabet for BinHex {
    const CHARSET: base64::alphabet::Alphabet = ::base64::alphabet::BIN_HEX;
}

pub trait Padding: sealed::Sealed {
    const CONFIG: GeneralPurposeConfig;
}

pub struct Padded;
impl Padding for Padded {
    const CONFIG: GeneralPurposeConfig = PAD;
}

pub struct Unpadded;
impl Padding for Unpadded {
    const CONFIG: GeneralPurposeConfig = NO_PAD;
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Standard {}
    impl Sealed for super::UrlSafe {}
    impl Sealed for super::Crypt {}
    impl Sealed for super::Bcrypt {}
    impl Sealed for super::ImapMutf7 {}
    impl Sealed for super::BinHex {}
    impl Sealed for super::Padded {}
    impl Sealed for super::Unpadded {}
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
        assert_roundtrip::<Base64, _>(v);
    }

    #[rstest]
    #[case::empty(b"")]
    #[case::dot(b".")]
    #[case::test(b"test")]
    #[case::hello(b"Hello, Near!")]
    #[case::hash(&[42u8; 32])]
    fn roundtrip_slice(#[case] v: &[u8]) {
        assert_roundtrip::<Base64, _>(v);
    }

    #[rstest]
    #[case::empty("")]
    #[case::dot(".")]
    #[case::test("test")]
    #[case::hello("Hello, Near!")]
    fn roundtrip_str(#[case] v: &str) {
        assert_roundtrip::<Base64, _>(v);
    }

    #[rstest]
    #[case::whitespace(b" ")]
    #[case::newline(b"\n")]
    #[case::leading_whitespace(b" AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=")]
    #[case::trailing_whitespace(b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA= ")]
    fn invalid_input(#[case] bytes: &[u8]) {
        <Base64 as IoDecoder<Vec<u8>>>::from_slice(bytes).unwrap_err();
    }
}

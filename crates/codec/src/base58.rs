//! Base58 encoding

use std::{io, marker::PhantomData};

use crate::{IoDecoder, IoEncoder};

/// Base58 codec.
///
/// # Example
///
/// ```rust
/// use defuse_codec::{IoDecoder, IoEncoder, base58::Base58};
///
/// let value = vec![17u8; 32];
/// let encoded = <Base58>::to_vec(&value).unwrap();
/// assert_eq!(encoded, b"29d2S7vB453rNYFdR5Ycwt7y9haRT5fwVwL9zTmBhfV2");
///
/// let decoded: Vec<u8> = <Base58>::from_slice(encoded).unwrap();
/// assert_eq!(decoded, value);
/// ```
pub struct Base58<ALPHABET: Alphabet = Standard>(PhantomData<ALPHABET>);

impl<T, ALPHABET> IoEncoder<T> for Base58<ALPHABET>
where
    T: AsRef<[u8]> + ?Sized,
    ALPHABET: Alphabet,
{
    #[inline]
    fn to_writer<W>(mut writer: W, value: &T) -> io::Result<()>
    where
        W: io::Write,
    {
        writer.write_all(&::bs58::encode::EncodeBuilder::new(value, ALPHABET::CHARSET).into_vec())
    }

    #[inline]
    fn to_vec(value: &T) -> io::Result<Vec<u8>> {
        Ok(::bs58::encode::EncodeBuilder::new(value, ALPHABET::CHARSET).into_vec())
    }
}

impl<T, ALPHABET> IoDecoder<T> for Base58<ALPHABET>
where
    T: TryFrom<Vec<u8>>, // TODO: remove display bound
    ALPHABET: Alphabet,
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
        let buf = ::bs58::decode::DecodeBuilder::new(bytes, ALPHABET::CHARSET)
            .into_vec()
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

/// A base58 alphabet
pub trait Alphabet: sealed::Sealed {
    /// A specific alphabet.
    const CHARSET: &'static ::bs58::Alphabet;
}

/// The default alphabet used if none is given. Currently is the
/// [`Bitcoin`] alphabet.
pub type Standard = Bitcoin;

/// Bitcoin's alphabet as defined in their `Base58Check` encoding.
///
/// See <https://en.bitcoin.it/wiki/Base58Check_encoding#Base58_symbol_chart>
pub struct Bitcoin;
impl Alphabet for Bitcoin {
    const CHARSET: &'static bs58::Alphabet = ::bs58::Alphabet::BITCOIN;
}

/// Monero's alphabet as defined in this forum post.
///
/// See <https://forum.getmonero.org/4/academic-and-technical/221/creating-a-standard-for-physical-coins>
pub struct Monero;
impl Alphabet for Monero {
    const CHARSET: &'static bs58::Alphabet = ::bs58::Alphabet::MONERO;
}

/// Ripple's alphabet as defined in their wiki.
///
/// See <https://wiki.ripple.com/Encodings>
pub struct Ripple;
impl Alphabet for Ripple {
    const CHARSET: &'static bs58::Alphabet = ::bs58::Alphabet::RIPPLE;
}

/// Flickr's alphabet for creating short urls from photo ids.
///
/// See <https://www.flickr.com/groups/api/discuss/72157616713786392/>
pub struct Flickr;
impl Alphabet for Flickr {
    const CHARSET: &'static bs58::Alphabet = ::bs58::Alphabet::FLICKR;
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Bitcoin {}
    impl Sealed for super::Monero {}
    impl Sealed for super::Ripple {}
    impl Sealed for super::Flickr {}
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
        assert_roundtrip::<Base58, _>(v);
    }

    #[rstest]
    #[case::empty(b"")]
    #[case::dot(b".")]
    #[case::test(b"test")]
    #[case::hello(b"Hello, Near!")]
    #[case::hash(&[42u8; 32])]
    fn roundtrip_slice(#[case] v: &[u8]) {
        assert_roundtrip::<Base58, _>(v);
    }

    #[rstest]
    #[case::empty("")]
    #[case::dot(".")]
    #[case::test("test")]
    #[case::hello("Hello, Near!")]
    fn roundtrip_str(#[case] v: &str) {
        assert_roundtrip::<Base58, _>(v);
    }

    #[rstest]
    #[case::whitespace(b" ")]
    #[case::newline(b"\n")]
    #[case::leading_whitespace(b" 29d2S7vB453rNYFdR5Ycwt7y9haRT5fwVwL9zTmBhfV2")]
    #[case::trailing_whitespace(b"29d2S7vB453rNYFdR5Ycwt7y9haRT5fwVwL9zTmBhfV2 ")]
    fn invalid_input(#[case] bytes: &[u8]) {
        <Base58 as IoDecoder<Vec<u8>>>::from_slice(bytes).unwrap_err();
    }
}

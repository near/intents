//! Base58 encoding

use std::{fmt::Display, io};

use crate::{IoDecoder, IoEncoder};

/// Base58 codec.
///
/// # Example
///
/// ```rust
/// use defuse_codec::{IoDecoder, IoEncoder, base58::Base58};
///
/// let value = vec![17u8; 32];
/// let encoded = Base58::to_vec(&value).unwrap();
/// assert_eq!(encoded, b"29d2S7vB453rNYFdR5Ycwt7y9haRT5fwVwL9zTmBhfV2");
///
/// let decoded: Vec<u8> = Base58::from_slice(encoded).unwrap();
/// assert_eq!(decoded, value);
/// ```
pub struct Base58;

impl<T> IoEncoder<T> for Base58
where
    T: AsRef<[u8]> + ?Sized,
{
    #[inline]
    fn to_writer<W>(mut writer: W, value: &T) -> io::Result<()>
    where
        W: io::Write,
    {
        // TODO: optimize to avoid allocations
        writer.write_all(&bs58::encode(value).into_vec())
    }

    #[inline]
    fn to_vec(value: &T) -> io::Result<Vec<u8>> {
        Ok(bs58::encode(value).into_vec())
    }
}

impl<T> IoDecoder<T> for Base58
where
    T: TryFrom<Vec<u8>, Error: Display>,
{
    #[inline]
    fn from_reader<R>(mut reader: R) -> io::Result<T>
    where
        R: io::Read,
    {
        // TODO: optimize to avoid this allocation?
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        Self::from_slice(buf)
    }

    #[inline]
    fn from_slice(bytes: impl AsRef<[u8]>) -> io::Result<T> {
        let buf = bs58::decode(bytes)
            .into_vec()
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        T::try_from(buf).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
    }
}

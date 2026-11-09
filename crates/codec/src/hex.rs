//! Hex encoding

use std::{fmt::Display, io};

use crate::{IoDecoder, IoEncoder};

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
pub struct Hex;

impl<T> IoEncoder<T> for Hex
where
    T: AsRef<[u8]> + ?Sized,
{
    #[inline]
    fn to_writer<W>(mut writer: W, value: &T) -> io::Result<()>
    where
        W: io::Write,
    {
        writer.write_all(hex::encode(value).as_bytes())
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
    T: TryFrom<Vec<u8>, Error: Display>,
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
        T::try_from(buf).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
    }
}

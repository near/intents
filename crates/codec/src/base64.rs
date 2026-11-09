//! Base64 encoding

use std::{
    fmt::Display,
    io::{self, Read, Write},
};

use base64::{Engine, read::DecoderReader, write::EncoderWriter};

use crate::{IoDecoder, IoEncoder};

/// Base64 codec.
///
/// # Example
///
/// ```rust
/// use defuse_codec::{IoDecoder, IoEncoder, base64::Base64};
///
/// let value = vec![0u8; 32];
/// let encoded = Base64::to_vec(&value).unwrap();
/// assert_eq!(encoded, b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
///
/// let decoded: Vec<u8> = Base64::from_slice(encoded).unwrap();
/// assert_eq!(decoded, value);
/// ```
// TODO: add support for:
// * custom alphabets
// * Pad/NoPad? strict?
pub struct Base64;

impl Base64 {
    #[inline]
    const fn engine() -> &'static impl Engine {
        // TODO: engine?
        &base64::engine::general_purpose::STANDARD
    }
}

impl<T> IoEncoder<T> for Base64
where
    T: AsRef<[u8]> + ?Sized,
{
    #[inline]
    fn to_writer<W>(writer: W, value: &T) -> io::Result<()>
    where
        W: io::Write,
    {
        let mut writer = EncoderWriter::new(writer, Self::engine());
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

impl<T> IoDecoder<T> for Base64
where
    T: TryFrom<Vec<u8>, Error: Display>,
{
    #[inline]
    fn from_reader<R>(reader: R) -> io::Result<T>
    where
        R: io::Read,
    {
        let mut reader = DecoderReader::new(reader, Self::engine());

        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        T::try_from(buf).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
    }

    #[inline]
    fn from_slice(bytes: impl AsRef<[u8]>) -> io::Result<T> {
        let buf = Self::engine()
            .decode(bytes)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        T::try_from(buf).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
    }
}

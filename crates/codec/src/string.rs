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

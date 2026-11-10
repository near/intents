use std::io;

use crate::{IoDecoder, IoEncoder};

/// A codec that encodes using `AsRef<[u8]>` implementation and decodes via `TryFrom<Vec<u8>>`.
///
/// # Example
///
/// ```rust
/// use defuse_codec::{IoDecoder, IoEncoder, AsBytes};
///
/// let value = "abc";
/// let encoded = AsBytes::to_vec(&value).unwrap();
/// assert_eq!(encoded, b"abc");
///
/// let decoded: String = AsBytes::from_slice(encoded).unwrap();
/// assert_eq!(decoded, value);
/// ```
pub struct AsBytes;

impl AsBytes {
    #[inline]
    pub(crate) fn try_from_vec<T>(v: Vec<u8>) -> io::Result<T>
    where
        T: TryFrom<Vec<u8>>,
    {
        let length = v.len();
        T::try_from(v).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("can't convert a byte vector of length {length} to the output type"),
            )
        })
    }
}

impl<T> IoEncoder<T> for AsBytes
where
    T: AsRef<[u8]> + ?Sized,
{
    #[inline]
    fn to_writer<W>(mut writer: W, value: &T) -> io::Result<()>
    where
        W: io::Write,
    {
        writer.write_all(value.as_ref())
    }

    #[inline]
    fn encoded_len(value: &T) -> io::Result<usize> {
        Ok(value.as_ref().len())
    }
}

impl<T> IoDecoder<T> for AsBytes
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
        Self::try_from_vec(buf)
    }
}

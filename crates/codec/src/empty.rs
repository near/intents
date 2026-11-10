use std::io::{self, Error, ErrorKind};

use crate::{ERR_NOT_EMPTY, IoDecoder, IoEncoder};

/// A codec for empty data.
///
/// Encoding produces no bytes, while decoding expects the input to be empty.
///
/// # Examples
///
/// ```rust
/// use defuse_codec::{IoDecoder, IoEncoder, Empty};
///
/// let encoded = Empty::to_vec(&()).unwrap();
/// assert!(encoded.is_empty());
///
/// let () = Empty::from_slice(b"").unwrap();
///
/// Empty::from_slice(b"not empty").expect_err("input must be empty");
/// ```
pub struct Empty;

impl IoEncoder<()> for Empty {
    #[inline]
    fn to_writer<W>(_writer: W, _value: &()) -> io::Result<()>
    where
        W: io::Write,
    {
        Ok(())
    }

    #[inline]
    fn encoded_len(_value: &()) -> io::Result<usize> {
        Ok(0)
    }
}

impl IoDecoder<()> for Empty {
    #[inline]
    fn from_reader<R>(reader: R) -> io::Result<()>
    where
        R: io::Read,
    {
        #[allow(clippy::unbuffered_bytes)]
        match reader.bytes().next() {
            None => Ok(()),
            Some(Ok(_)) => Err(Error::new(ErrorKind::InvalidData, ERR_NOT_EMPTY)),
            Some(Err(err)) => Err(err),
        }
    }

    #[inline]
    fn from_slice(bytes: impl AsRef<[u8]>) -> io::Result<()> {
        if !bytes.as_ref().is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, ERR_NOT_EMPTY));
        }

        Ok(())
    }
}

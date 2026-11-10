//! Borsh binary serialization format.

use std::{io, marker::PhantomData};

use defuse_borsh_utils::{BorshDeserializeAs, BorshSerializeAs, Same};

use crate::{ERR_TRAILING_BYTES, IoDecoder, IoEncoder};

/// A codec for borsh.
///
/// # Example
///
/// ```rust
/// use defuse_codec::{IoDecoder, IoEncoder, borsh::Borsh};
///
/// let value = "abc";
/// let encoded = <Borsh>::to_vec(&value).unwrap();
/// assert_eq!(encoded, b"\x03\x00\x00\x00abc");
///
/// let decoded: String = <Borsh>::from_slice(encoded).unwrap();
/// assert_eq!(decoded, value);
/// ```
pub struct Borsh<As: ?Sized = Same>(PhantomData<As>);

impl<T, As> IoEncoder<T> for Borsh<As>
where
    T: ?Sized,
    As: BorshSerializeAs<T> + ?Sized,
{
    #[inline]
    fn to_writer<W>(mut writer: W, value: &T) -> io::Result<()>
    where
        W: io::Write,
    {
        As::serialize_as(value, &mut writer)
    }
}

impl<T, As> IoDecoder<T> for Borsh<As>
where
    As: BorshDeserializeAs<T> + ?Sized,
{
    #[inline]
    fn from_reader<R>(mut reader: R) -> io::Result<T>
    where
        R: io::Read,
    {
        let value = As::deserialize_reader_as(&mut reader)?;
        #[allow(clippy::unbuffered_bytes)]
        match reader.bytes().next() {
            None => Ok(value),
            Some(Ok(_)) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                ERR_TRAILING_BYTES,
            )),
            Some(Err(err)) => Err(err),
        }
    }

    #[inline]
    fn from_slice(bytes: impl AsRef<[u8]>) -> io::Result<T> {
        let mut buf = bytes.as_ref();
        let value = As::deserialize_as(&mut buf)?;
        if !buf.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                ERR_TRAILING_BYTES,
            ));
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::empty(&[])]
    #[case::excessive(&[1, 2])]
    fn invalid_input_u8(#[case] bytes: &[u8]) {
        <Borsh as IoDecoder<u8>>::from_slice(bytes).unwrap_err();
    }
}

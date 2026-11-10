//! Borsh binary serialization format.

use std::{io, marker::PhantomData};

use defuse_borsh_utils::{AsWrap, BorshDeserializeAs, BorshSerializeAs, Same};

use crate::{ERR_NOT_EMPTY, IoDecoder, IoEncoder};

/// A codec for [borsh].
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
    fn to_writer<W>(writer: W, value: &T) -> io::Result<()>
    where
        W: io::Write,
    {
        borsh::to_writer(writer, &AsWrap::<&T, &As>::new(value))
    }

    #[inline]
    fn to_vec(value: &T) -> io::Result<Vec<u8>> {
        borsh::to_vec(&AsWrap::<&T, &As>::new(value))
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
            Some(Ok(_)) => Err(io::Error::new(io::ErrorKind::InvalidData, ERR_NOT_EMPTY)),
            Some(Err(err)) => Err(err),
        }
    }

    #[inline]
    fn from_slice(bytes: impl AsRef<[u8]>) -> io::Result<T> {
        let mut bytes = bytes.as_ref();
        let value = As::deserialize_as(&mut bytes)?;
        if !bytes.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, ERR_NOT_EMPTY));
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

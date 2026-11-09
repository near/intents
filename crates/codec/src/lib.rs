//! Codecs (i.e. **data formats**) for [encoding](IoEncoder) and [decoding](IoDecoder) data.
#![cfg_attr(
    feature = "document-features",
    cfg_attr(
        doc,
        doc = "## Feature flags",
        doc = ::document_features::document_features!(feature_label = r#"<span class="stab portability"><code>{feature}</code></span>"#),
    )
)]

#[cfg(feature = "base58")]
pub mod base58;
#[cfg(feature = "base64")]
pub mod base64;
#[cfg(feature = "borsh")]
pub mod borsh;
mod bytes;
mod empty;
#[cfg(feature = "hex")]
pub mod hex;
#[cfg(feature = "json")]
pub mod json;
mod string;

pub use self::{bytes::*, empty::*, string::*};

use std::io;

/// A **data format** for encoding values of type `T`.
pub trait IoEncoder<T: ?Sized> {
    /// Encode a value directly to writer.
    ///
    /// # Example
    ///
    /// ```rust
    /// use defuse_codec::{IoEncoder, borsh::Borsh};
    ///
    /// let mut buf = Vec::new();
    /// <Borsh>::to_writer(&mut buf, &12u64).unwrap();
    ///
    /// assert_eq!(buf, vec![12, 0, 0, 0, 0, 0, 0, 0]);
    /// ```
    fn to_writer<W>(writer: W, value: &T) -> io::Result<()>
    where
        W: io::Write;

    /// Encode a value into a vector of bytes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use defuse_codec::{IoEncoder, borsh::Borsh};
    ///
    /// let encoded = <Borsh>::to_vec(&12u64).unwrap();
    /// assert_eq!(encoded, vec![12, 0, 0, 0, 0, 0, 0, 0]);
    /// ```
    #[inline]
    fn to_vec(value: &T) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        Self::to_writer(&mut buf, value)?;
        Ok(buf)
    }

    /// Returns the exact length (in bytes) of the to-be-encoded value.
    ///
    /// Default implementation [encodes](Self::to_writer) given value to a
    /// counting writer without allocations, but implementations MAY use a
    /// more optimized approach.
    ///
    /// # Example
    ///
    /// ```rust
    /// use defuse_codec::{IoEncoder, borsh::Borsh};
    ///
    /// let len = <Borsh>::encoded_len("abc").unwrap();
    /// assert_eq!(len, 7); // <u32 length> + b"abc"
    /// ```
    #[inline]
    fn encoded_len(value: &T) -> io::Result<usize> {
        struct Counter(usize);
        impl io::Write for Counter {
            #[inline]
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                self.0 = self
                    .0
                    .checked_add(buf.len())
                    .ok_or(io::ErrorKind::OutOfMemory)?;
                Ok(buf.len())
            }
            #[inline]
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let mut counter = Counter(0);
        Self::to_writer(&mut counter, value)?;
        Ok(counter.0)
    }
}

/// A **data format** for decoding values of type `T`.
///
/// Each decoding method treats its input as a complete encoding of one value.
/// Implementations MUST validate the **entire** input according to the codec's
/// rules and reject unexpected trailing data.
pub trait IoDecoder<T> {
    /// Decode a value from the complete input supplied by `reader`.
    ///
    /// On success, reader MUST be consumed until EOF. Trailing data is permitted
    /// only when codec's rules allow it, and MUST also be consumed.
    ///
    /// This method MAY block waiting for EOF. For persistent streams, supply a
    /// reader bounded to one message.
    ///
    /// It's recommended to use [`from_str`](Self::from_str) or [`from_slice`](Self::from_slice)
    /// when the data is already loaded in-memory.
    ///
    /// # Errors
    ///
    /// Returns an error if the input is invalid, including unexpected trailing
    /// data, or any other read error encountered.
    ///
    /// On error, the reader MAY be partially consumed.
    ///
    /// # Example
    ///
    /// ```rust
    /// use defuse_codec::{IoDecoder, borsh::Borsh};
    ///
    /// let buf: &[u8] = &[12, 0, 0, 0, 0, 0, 0, 0];
    /// let value: u64 = <Borsh>::from_reader(buf).unwrap();
    /// assert_eq!(value, 12);
    /// ```
    fn from_reader<R>(reader: R) -> io::Result<T>
    where
        R: io::Read;

    /// Decode a value from the entire slice of bytes.
    ///
    /// Applies the same format and validation rules as [`from_reader`](Self::from_reader).
    ///
    /// In contrast to [`from_reader`](Self::from_reader), this method allows optimized
    /// implementations to avoid unnecessary allocations when possible. However, when 
    ///
    /// It's recommended to use [`from_str`](Self::from_str) when `&str` is already available:
    /// this allows optimized implementations to also skip UTF-8 validation when needed.
    ///
    /// # Errors
    ///
    /// Returns an error if the input is invalid according to the codec's rules,
    /// including trailing data that those rules do not permit.
    ///
    /// # Example
    ///
    /// ```rust
    /// use defuse_codec::{IoDecoder, borsh::Borsh};
    ///
    /// let value: u64 = <Borsh>::from_slice([12, 0, 0, 0, 0, 0, 0, 0]).unwrap();
    /// assert_eq!(value, 12);
    /// ```
    #[inline]
    fn from_slice(bytes: impl AsRef<[u8]>) -> io::Result<T> {
        Self::from_reader(bytes.as_ref())
    }

    /// Decodes a value from the entire string.
    ///
    /// Applies the same format and validation rules as [`from_slice`](Self::from_slice).
    ///
    /// In contrast to [`from_slice`](Self::from_slice), this method allows optimized
    /// implementations to skip UTF-8 validation when needed.
    ///
    /// # Errors
    ///
    /// Returns an error if the input is invalid according to the codec's rules,
    /// including trailing data that those rules do not permit.
    ///
    /// # Example
    ///
    /// ```rust
    /// use defuse_codec::{IoDecoder, DisplayFromStr};
    ///
    /// let value: u8 = DisplayFromStr::from_str("42").unwrap();
    /// assert_eq!(value, 42);
    /// ```
    #[inline]
    fn from_str(s: impl AsRef<str>) -> io::Result<T> {
        Self::from_slice(s.as_ref())
    }
}

#[cfg(test)]
mod test_utils;

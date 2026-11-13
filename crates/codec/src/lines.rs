use std::{
    io::{self, BufReader},
    iter,
    marker::PhantomData,
};

use crate::{DisplayFromStr, IoDecoder, IoEncoder};

/// A codec for collection of items encoded as separate UTF-8 lines using the inner codec `T`.
///
/// Each encoded item MUST be a valid UTF-8 and MUST NOT contain `\n` or end with '\r'.
/// A newline is written after every item, including empty items.
///
/// Decoding accepts LF and CRLF line endings, following [`str::lines`].
///
/// # Example
///
/// ```rust
/// use defuse_codec::{IoDecoder, IoEncoder, Lines};
///
/// let value: Vec<u32> = vec![1, 23, 456];
/// let encoded = <Lines>::to_string(&value).unwrap();
/// assert_eq!(encoded, "1\n23\n456\n");
///
/// let decoded: Vec<u32> = <Lines>::from_str(encoded).unwrap();
/// assert_eq!(decoded, value);
/// ```
pub struct Lines<T: ?Sized = DisplayFromStr>(PhantomData<T>);

impl<E: ?Sized> Lines<E> {
    fn encode_line<T>(mut buf: &mut Vec<u8>, value: &T) -> io::Result<()>
    where
        E: IoEncoder<T>,
    {
        let l = buf.len();
        E::to_writer(&mut buf, value)?;
        // `impl io::Writer for Vec<u8>` only appends to end, so we need to check only appended part
        let s = str::from_utf8(&buf[l..])
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        if s.as_bytes().contains(&b'\n') || s.as_bytes().ends_with(b"\r") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "encoded UTF-8 line MUST NOT contain LF or end with CR",
            ));
        }
        buf.push(b'\n');
        Ok(())
    }
}

impl<I, E> IoEncoder<I> for Lines<E>
where
    for<'a> &'a I: IntoIterator,
    E: ?Sized + for<'a> IoEncoder<<&'a I as IntoIterator>::Item>,
    I: ?Sized,
{
    fn to_writer<W>(mut writer: W, value: &I) -> io::Result<()>
    where
        W: io::Write,
    {
        let mut buf = Vec::new();
        for item in value {
            buf.clear();
            Self::encode_line(&mut buf, &item)?;
            writer.write_all(&buf)?;
        }
        Ok(())
    }

    fn to_vec(value: &I) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        for item in value {
            Self::encode_line(&mut buf, &item)?;
        }
        Ok(buf)
    }

    #[inline]
    fn to_string(value: &I) -> io::Result<String> {
        let vec = Self::to_vec(value)?;
        Ok(
            // SAFETY: `Self::to_vec` validates each appended item as UTF-8
            // and adds only ASCII newlines.
            unsafe { String::from_utf8_unchecked(vec) },
        )
    }
}

impl<I, D> IoDecoder<I> for Lines<D>
where
    I: IntoIterator + FromIterator<<I as IntoIterator>::Item>,
    D: IoDecoder<I::Item> + ?Sized,
{
    #[inline]
    fn from_reader<R>(reader: R) -> io::Result<I>
    where
        R: io::Read,
    {
        Self::from_buf_read(BufReader::new(reader))
    }

    #[inline]
    fn from_buf_read<R>(mut reader: R) -> io::Result<I>
    where
        R: io::BufRead,
    {
        iter::from_fn({
            let mut buf = String::new();
            move || {
                buf.clear();
                match reader.read_line(&mut buf) {
                    Ok(0) => None,
                    Ok(_n) => {
                        if buf.ends_with('\n') {
                            buf.pop();
                            if buf.ends_with('\r') {
                                buf.pop();
                            }
                        }
                        Some(D::from_str(&buf))
                    }
                    Err(e) => Some(Err(e)),
                }
            }
        })
        .collect()
    }

    #[inline]
    fn from_slice(bytes: impl AsRef<[u8]>) -> io::Result<I> {
        str::from_utf8(bytes.as_ref())
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
            .and_then(Self::from_str)
    }

    #[inline]
    fn from_str(s: impl AsRef<str>) -> io::Result<I> {
        s.as_ref().lines().map(D::from_str).collect()
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::test_utils::assert_roundtrip;

    use super::*;

    #[rstest]
    #[case::empty(&[], "")]
    #[case::empty_item(&[""], "\n")]
    #[case::blank_lines(&["first", "", "last", ""], "first\n\nlast\n\n")]
    #[case::unicode(&["é", "Near 💖"], "é\nNear 💖\n")]
    #[case::carriage_return(&["a\rb"], "a\rb\n")]
    fn lines_roundtrip(#[case] input: &[&str], #[case] expected: &str) {
        assert_roundtrip::<Lines, Vec<String>>(
            &input.iter().copied().map(ToOwned::to_owned).collect(),
            Some(expected.as_bytes()),
        );
    }

    #[rstest]
    #[case::newline(b"invalid\nline")]
    #[case::last_carriage_return(b"line\r")]
    #[case::invalid_utf8(b"valid\xff")]
    #[case::incomplete_utf8(b"\xe2\x82")]
    fn lines_reject_invalid_item(#[case] item: &[u8]) {
        Lines::<crate::AsBytes>::to_vec(&[b"first".as_slice(), item, b"last"]).unwrap_err();
    }
}

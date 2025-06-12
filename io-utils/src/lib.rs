use std::io::{Read, Result, Write};

pub trait ReadExt: Read {
    /// Creates a `TeeReader` that wraps the current reader and duplicates all read bytes into the given writer.
    fn tee<W>(self, writer: W) -> TeeReader<Self, W>
    where
        Self: Sized,
        W: Write,
    {
        TeeReader {
            reader: self,
            writer,
        }
    }
}
impl<R> ReadExt for R where R: Read {}

/// A reader that wraps another reader and writes all bytes read into an internal writer.
pub struct TeeReader<R, W> {
    reader: R,
    writer: W,
}

impl<R, W> TeeReader<R, W> {
    #[inline]
    pub fn into_inner(self) -> (R, W) {
        (self.reader, self.writer)
    }
}

impl<R, W> Read for TeeReader<R, W>
where
    R: Read,
    W: Write,
{
    /// Reads data from the underlying reader into the buffer, and writes the same data to the writer.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = self.reader.read(buf)?;
        self.writer.write_all(&buf[..n])?;
        Ok(n)
    }

    /// Reads all remaining data into a buffer and writes it to the internal writer.
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let n = self.reader.read_to_end(buf)?;
        self.writer.write_all(&buf[..n])?;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};

    #[test]
    fn tee_read_copies_to_writer() {
        let input_data = b"hello world";
        let input = Cursor::new(input_data);
        let output = Cursor::new(Vec::new());

        let mut tee = input.tee(output);
        let mut buf = [0u8; 5];

        let n = tee.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"hello");

        let (mut input, output) = tee.into_inner();

        let written = output.into_inner();
        assert_eq!(&written[..n], b"hello");

        let mut rest = Vec::new();
        input.read_to_end(&mut rest).unwrap();
        assert_eq!(rest, b" world");
    }

    #[test]
    fn tee_read_to_end() {
        let input_data = b"stream this";
        let input = Cursor::new(input_data);
        let output = Cursor::new(Vec::new());

        let mut tee = input.tee(output);
        let mut buf = Vec::new();

        let n = tee.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, b"stream this");

        let (_input, output) = tee.into_inner();
        let written = output.into_inner();
        assert_eq!(&written[..n], b"stream this");
    }
}

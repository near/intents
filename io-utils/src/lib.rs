use std::io::{Read, Result, Write};

pub trait ReadExt: Read {
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
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = self.reader.read(buf)?;
        self.writer.write_all(&buf[..n])?;
        Ok(n)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let n = self.reader.read_to_end(buf)?;
        self.writer.write_all(&buf[..n])?;
        Ok(n)
    }
}

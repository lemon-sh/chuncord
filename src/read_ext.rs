use std::{io, io::Read};

pub trait ReadExt {
    fn read_max(&mut self, buf: &mut [u8]) -> Result<usize, io::Error>;
}

impl<T: Read> ReadExt for T {
    fn read_max(&mut self, mut buf: &mut [u8]) -> Result<usize, io::Error> {
        let mut bytes_read = 0;
        while !buf.is_empty() {
            match self.read(buf)? {
                0 => break,
                n => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                    bytes_read += n;
                }
            }
        }
        Ok(bytes_read)
    }
}

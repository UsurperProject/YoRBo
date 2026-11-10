use std::io::{self, Write};

pub struct ProgressWriter<W: Write, F: FnMut(u64)> {
    inner: W,
    written: u64,
    on_write: F,
}

impl<W: Write, F: FnMut(u64)> ProgressWriter<W, F> {
    pub fn new(inner: W, on_write: F) -> Self {
        Self { inner, written: 0, on_write }
    }
}

impl<W: Write, F: FnMut(u64)> Write for ProgressWriter<W, F> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.written += n as u64;
        (self.on_write)(self.written);
        io::Result::Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}


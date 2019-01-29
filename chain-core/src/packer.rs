//! Tooling for packing and unpacking from streams
//!
//! This will allow us to expose some standard way of serializing
//! data.

pub struct Codec<I>(I);
impl<I> Codec<I> {
    pub fn into_inner(self) -> I {
        self.0
    }
}
impl<R: std::io::BufRead> Codec<R> {
    pub fn get_u8(&mut self) -> std::io::Result<u8> {
        let mut buf = [0u8; 1];
        self.0.read_exact(&mut buf)?;
        Ok(buf[0])
    }
    pub fn get_u16(&mut self) -> std::io::Result<u16> {
        let mut buf = [0u8; 2];
        self.0.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }
    pub fn get_u32(&mut self) -> std::io::Result<u32> {
        let mut buf = [0u8; 4];
        self.0.read_exact(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }
    pub fn get_u64(&mut self) -> std::io::Result<u64> {
        let mut buf = [0u8; 8];
        self.0.read_exact(&mut buf)?;
        Ok(u64::from_be_bytes(buf))
    }
    pub fn get_u128(&mut self) -> std::io::Result<u128> {
        let mut buf = [0u8; 16];
        self.0.read_exact(&mut buf)?;
        Ok(u128::from_be_bytes(buf))
    }
}
impl<W: std::io::Write> Codec<W> {
    pub fn put_u8(&mut self, v: u8) -> std::io::Result<()> {
        self.0.write_all(&[v])
    }
    pub fn put_u16(&mut self, v: u16) -> std::io::Result<()> {
        self.0.write_all(&v.to_be_bytes())
    }
    pub fn put_u32(&mut self, v: u32) -> std::io::Result<()> {
        self.0.write_all(&v.to_be_bytes())
    }
    pub fn put_u64(&mut self, v: u64) -> std::io::Result<()> {
        self.0.write_all(&v.to_be_bytes())
    }
    pub fn put_u128(&mut self, v: u128) -> std::io::Result<()> {
        self.0.write_all(&v.to_be_bytes())
    }
}
impl<R: std::io::Read> std::io::Read for Codec<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}
impl<BR: std::io::BufRead> std::io::BufRead for Codec<BR> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.0.fill_buf()
    }
    fn consume(&mut self, amt: usize) {
        self.0.consume(amt)
    }
}
impl<W: std::io::Write> std::io::Write for Codec<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}
impl<I> From<I> for Codec<I> {
    fn from(inner: I) -> Self {
        Codec(inner)
    }
}

use chain_core::property;

/// Block Header Bytes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderRaw(pub(super) Vec<u8>);

impl AsRef<[u8]> for HeaderRaw {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl property::Serialize for HeaderRaw {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        use std::io::Write;

        let mut codec = Codec::new(writer);
        //dbg!(self.0.len());
        codec.put_u16(self.0.len() as u16)?;
        codec.write_all(&self.0)?;
        Ok(())
    }
}

impl property::Deserialize for HeaderRaw {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::Codec;
        use std::io::Read;

        let mut codec = Codec::new(reader);

        let header_size = codec.get_u16()? as usize;
        //dbg!(header_size);
        let mut v = vec![0u8; header_size];
        codec.read_exact(&mut v[..])?;
        Ok(HeaderRaw(v))
    }
}

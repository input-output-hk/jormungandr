use crate::value::Value;

pub use cardano::address::Addr as OldAddress;
use chain_core::property;

#[derive(Debug, Clone)]
pub struct UtxoDeclaration {
    pub protocol_magic: u32,
    pub addrs: Vec<(OldAddressBytes, Value)>,
}

type OldAddressBytes = Vec<u8>;

impl property::Deserialize for UtxoDeclaration {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        use std::io::Read;
        let mut codec = Codec::from(reader);
        let protocol_magic = codec.get_u32()?;
        let nb_entries = codec.get_u8()?;
        // FIXME add proper error
        assert!(nb_entries < 0xff);
        let mut addrs = Vec::with_capacity(nb_entries as usize);
        for _ in 0..nb_entries {
            let value = Value::deserialize(&mut codec)?;
            let addr_size = codec.get_u16()? as usize;
            let mut addr_buf = vec![0u8; addr_size];
            codec.read_exact(&mut addr_buf)?;
            addrs.push((addr_buf, value))
        }

        Ok(UtxoDeclaration {
            protocol_magic: protocol_magic,
            addrs: addrs,
        })
    }
}

impl property::Serialize for UtxoDeclaration {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        use std::io::Write;

        assert!(self.addrs.len() < 255);

        let mut codec = Codec::from(writer);
        codec.put_u32(self.protocol_magic)?;
        codec.put_u8(self.addrs.len() as u8)?;
        for (b, v) in &self.addrs {
            v.serialize(&mut codec)?;
            codec.put_u16(b.len() as u16)?;
            codec.write_all(&b)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for UtxoDeclaration {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let protocol_magic = Arbitrary::arbitrary(g);
            let mut nb: usize = Arbitrary::arbitrary(g);
            nb = nb % 255;
            let mut addrs = Vec::with_capacity(nb);
            for _ in 0..nb {
                let value = Arbitrary::arbitrary(g);
                let addr = vec![Arbitrary::arbitrary(g), 1u8];
                addrs.push((addr, value))
            }

            UtxoDeclaration {
                protocol_magic,
                addrs,
            }
        }
    }
}

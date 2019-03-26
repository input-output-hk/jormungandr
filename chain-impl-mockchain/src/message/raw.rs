use crate::key::Hash;
use chain_core::property;

// FIXME: should this be a wrapper type?
pub type MessageId = Hash;

/// A serialized Message
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageRaw(pub(super) Vec<u8>);

impl MessageRaw {
    pub fn size_bytes_plus_size(&self) -> usize {
        2 + self.0.len()
    }

    pub fn id(&self) -> MessageId {
        MessageId::hash_bytes(self.0.as_ref())
    }
}

impl AsRef<[u8]> for MessageRaw {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl property::Deserialize for MessageRaw {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(reader);
        let size = codec.get_u16()?;
        let mut v = vec![0u8; size as usize];
        codec.into_inner().read_exact(&mut v)?;
        Ok(MessageRaw(v))
    }
}

impl property::Serialize for MessageRaw {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;

        let mut codec = Codec::from(writer);
        codec.put_u16(self.0.len() as u16)?;
        codec.into_inner().write_all(&self.0)?;
        Ok(())
    }
}

mod raw;

use crate::legacy;
use chain_addr::Address;
use chain_core::mempack::{read_from_raw, ReadBuf, ReadError, Readable};
use chain_core::property;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

pub use raw::{MessageId, MessageRaw};

use crate::{
    certificate, setting,
    transaction::{AuthenticatedTransaction, NoExtra},
};

#[derive(Debug, Clone)]
pub enum Message {
    OldUtxoDeclaration(legacy::UtxoDeclaration),
    Transaction(AuthenticatedTransaction<Address, NoExtra>),
    Certificate(AuthenticatedTransaction<Address, certificate::Certificate>),
    Update(setting::UpdateProposal),
}

#[derive(Debug, Clone, Copy, FromPrimitive, PartialEq, Eq)]
enum MessageTag {
    OldUtxoDeclaration = 0,
    Transaction = 1,
    Certificate = 2,
    Update = 3,
}

impl Message {
    pub fn to_raw(&self) -> MessageRaw {
        use chain_core::packer::*;
        use chain_core::property::Serialize;
        let v = Vec::new();
        let mut codec = Codec::from(v);
        match self {
            Message::OldUtxoDeclaration(s) => {
                codec.put_u8(MessageTag::OldUtxoDeclaration as u8).unwrap();
                s.serialize(&mut codec).unwrap();
            }
            Message::Transaction(signed) => {
                codec.put_u8(MessageTag::Transaction as u8).unwrap();
                signed.serialize(&mut codec).unwrap();
            }
            Message::Certificate(signed) => {
                codec.put_u8(MessageTag::Certificate as u8).unwrap();
                signed.serialize(&mut codec).unwrap();
            }
            Message::Update(proposal) => {
                codec.put_u8(MessageTag::Update as u8).unwrap();
                proposal.serialize(&mut codec).unwrap();
            }
        }
        MessageRaw(codec.into_inner())
    }
}

impl property::Serialize for Message {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        let raw = self.to_raw();
        writer.write_all(raw.as_ref())
    }
}

impl Readable for Message {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let tag = buf.get_u8()?;
        match MessageTag::from_u8(tag) {
            Some(MessageTag::OldUtxoDeclaration) => {
                legacy::UtxoDeclaration::read(buf).map(Message::OldUtxoDeclaration)
            }
            Some(MessageTag::Transaction) => {
                AuthenticatedTransaction::read(buf).map(Message::Transaction)
            }
            Some(MessageTag::Certificate) => {
                AuthenticatedTransaction::read(buf).map(Message::Certificate)
            }
            Some(MessageTag::Update) => setting::UpdateProposal::read(buf).map(Message::Update),
            None => Err(ReadError::UnknownTag(tag as u32)),
        }
    }
}

impl property::Deserialize for Message {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        let raw = MessageRaw::deserialize(reader)?;
        read_from_raw(raw.as_ref())
    }
}

impl property::Message for Message {
    type Id = MessageId;

    /// The ID of a message is a hash of its serialization *without* the size.
    fn id(&self) -> Self::Id {
        self.to_raw().id()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Message {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            match g.next_u32() % 4 {
                0 => Message::OldUtxoDeclaration(Arbitrary::arbitrary(g)),
                1 => Message::Transaction(Arbitrary::arbitrary(g)),
                2 => Message::Certificate(Arbitrary::arbitrary(g)),
                _ => Message::Update(Arbitrary::arbitrary(g)),
            }
        }
    }
}

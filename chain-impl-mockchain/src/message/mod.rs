pub mod initial;
mod raw;

use crate::legacy;
use chain_addr::Address;
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

pub use initial::InitialEnts;
pub use raw::{MessageId, MessageRaw};

use crate::{
    certificate, setting,
    transaction::{AuthenticatedTransaction, NoExtra},
};

/// All possible messages recordable in the content
#[derive(Debug, Clone)]
pub enum Message {
    Initial(InitialEnts),
    OldUtxoDeclaration(legacy::UtxoDeclaration),
    Transaction(AuthenticatedTransaction<Address, NoExtra>),
    Certificate(AuthenticatedTransaction<Address, certificate::Certificate>),
    UpdateProposal(setting::UpdateProposal),
    UpdateVote(setting::SignedUpdateVote),
}

/// Tag enumeration of all known message
#[derive(Debug, Clone, Copy, FromPrimitive, PartialEq, Eq)]
pub(super) enum MessageTag {
    Initial = 0,
    OldUtxoDeclaration = 1,
    Transaction = 2,
    Certificate = 3,
    UpdateProposal = 4,
    UpdateVote = 5,
}

impl Message {
    /// Return the tag associated with the Message
    pub(super) fn get_tag(&self) -> MessageTag {
        match self {
            Message::Initial(_) => MessageTag::Initial,
            Message::OldUtxoDeclaration(_) => MessageTag::OldUtxoDeclaration,
            Message::Transaction(_) => MessageTag::Transaction,
            Message::Certificate(_) => MessageTag::Certificate,
            Message::UpdateProposal(_) => MessageTag::UpdateProposal,
            Message::UpdateVote(_) => MessageTag::UpdateVote,
        }
    }

    /// Get the serialized representation of this message
    pub fn to_raw(&self) -> MessageRaw {
        use chain_core::packer::*;
        use chain_core::property::Serialize;
        let v = Vec::new();
        let mut codec = Codec::from(v);
        codec.put_u8(self.get_tag() as u8).unwrap();
        match self {
            Message::Initial(i) => i.serialize(&mut codec).unwrap(),
            Message::OldUtxoDeclaration(s) => s.serialize(&mut codec).unwrap(),
            Message::Transaction(signed) => signed.serialize(&mut codec).unwrap(),
            Message::Certificate(signed) => signed.serialize(&mut codec).unwrap(),
            Message::UpdateProposal(proposal) => proposal.serialize(&mut codec).unwrap(),
            Message::UpdateVote(vote) => vote.serialize(&mut codec).unwrap(),
        }
        MessageRaw(codec.into_inner())
    }

    pub fn from_raw(raw: &MessageRaw) -> Result<Self, ReadError> {
        let buf = &mut ReadBuf::from(raw.as_ref());
        let tag = buf.get_u8()?;
        match MessageTag::from_u8(tag) {
            Some(MessageTag::Initial) => InitialEnts::read(buf).map(Message::Initial),
            Some(MessageTag::OldUtxoDeclaration) => {
                legacy::UtxoDeclaration::read(buf).map(Message::OldUtxoDeclaration)
            }
            Some(MessageTag::Transaction) => {
                AuthenticatedTransaction::read(buf).map(Message::Transaction)
            }
            Some(MessageTag::Certificate) => {
                AuthenticatedTransaction::read(buf).map(Message::Certificate)
            }
            Some(MessageTag::UpdateProposal) => {
                setting::UpdateProposal::read(buf).map(Message::UpdateProposal)
            }
            Some(MessageTag::UpdateVote) => {
                setting::SignedUpdateVote::read(buf).map(Message::UpdateVote)
            }
            None => Err(ReadError::UnknownTag(tag as u32)),
        }
    }
}

impl property::Serialize for Message {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        self.to_raw().serialize(writer)
    }
}

impl property::Deserialize for Message {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        let raw = MessageRaw::deserialize(reader)?;
        Message::from_raw(&raw)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
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
            match g.next_u32() % 6 {
                0 => Message::Initial(Arbitrary::arbitrary(g)),
                1 => Message::OldUtxoDeclaration(Arbitrary::arbitrary(g)),
                2 => Message::Transaction(Arbitrary::arbitrary(g)),
                3 => Message::Certificate(Arbitrary::arbitrary(g)),
                4 => Message::UpdateProposal(Arbitrary::arbitrary(g)),
                _ => Message::UpdateVote(Arbitrary::arbitrary(g)),
            }
        }
    }
}

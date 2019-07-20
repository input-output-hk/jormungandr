pub mod config;
mod raw;

use crate::legacy;
use chain_addr::Address;
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property;

pub use config::ConfigParams;
pub use raw::{FragmentId, FragmentRaw};

use crate::{
    certificate,
    transaction::{AuthenticatedTransaction, NoExtra},
    update::{SignedUpdateProposal, SignedUpdateVote},
};

/// Old name for Fragment. (soft) deprecated
pub type Message = Fragment;

/// Old name for FragmentTag. (soft) deprecated
pub(super) type MessageTag = FragmentTag;

/// All possible messages recordable in the content
#[derive(Debug, Clone)]
pub enum Fragment {
    Initial(ConfigParams),
    OldUtxoDeclaration(legacy::UtxoDeclaration),
    Transaction(AuthenticatedTransaction<Address, NoExtra>),
    Certificate(AuthenticatedTransaction<Address, certificate::Certificate>),
    UpdateProposal(SignedUpdateProposal),
    UpdateVote(SignedUpdateVote),
}

/// Tag enumeration of all known fragment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FragmentTag {
    Initial = 0,
    OldUtxoDeclaration = 1,
    Transaction = 2,
    Certificate = 3,
    UpdateProposal = 4,
    UpdateVote = 5,
}

impl FragmentTag {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(FragmentTag::Initial),
            1 => Some(FragmentTag::OldUtxoDeclaration),
            2 => Some(FragmentTag::Transaction),
            3 => Some(FragmentTag::Certificate),
            4 => Some(FragmentTag::UpdateProposal),
            5 => Some(FragmentTag::UpdateVote),
            _ => None,
        }
    }
}

impl Fragment {
    /// Return the tag associated with the Message
    pub(super) fn get_tag(&self) -> MessageTag {
        match self {
            Fragment::Initial(_) => FragmentTag::Initial,
            Fragment::OldUtxoDeclaration(_) => FragmentTag::OldUtxoDeclaration,
            Fragment::Transaction(_) => FragmentTag::Transaction,
            Fragment::Certificate(_) => FragmentTag::Certificate,
            Fragment::UpdateProposal(_) => FragmentTag::UpdateProposal,
            Fragment::UpdateVote(_) => FragmentTag::UpdateVote,
        }
    }

    /// Get the serialized representation of this message
    pub fn to_raw(&self) -> FragmentRaw {
        use chain_core::packer::*;
        use chain_core::property::Serialize;
        let v = Vec::new();
        let mut codec = Codec::new(v);
        codec.put_u8(self.get_tag() as u8).unwrap();
        match self {
            Fragment::Initial(i) => i.serialize(&mut codec).unwrap(),
            Fragment::OldUtxoDeclaration(s) => s.serialize(&mut codec).unwrap(),
            Fragment::Transaction(signed) => signed.serialize(&mut codec).unwrap(),
            Fragment::Certificate(signed) => signed.serialize(&mut codec).unwrap(),
            Fragment::UpdateProposal(proposal) => proposal.serialize(&mut codec).unwrap(),
            Fragment::UpdateVote(vote) => vote.serialize(&mut codec).unwrap(),
        }
        FragmentRaw(codec.into_inner())
    }

    pub fn from_raw(raw: &FragmentRaw) -> Result<Self, ReadError> {
        let mut buf = ReadBuf::from(raw.as_ref());
        Fragment::read(&mut buf)
    }
}

impl Readable for Fragment {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let tag = buf.get_u8()?;
        match FragmentTag::from_u8(tag) {
            Some(FragmentTag::Initial) => ConfigParams::read(buf).map(Fragment::Initial),
            Some(FragmentTag::OldUtxoDeclaration) => {
                legacy::UtxoDeclaration::read(buf).map(Fragment::OldUtxoDeclaration)
            }
            Some(FragmentTag::Transaction) => {
                AuthenticatedTransaction::read(buf).map(Fragment::Transaction)
            }
            Some(FragmentTag::Certificate) => {
                AuthenticatedTransaction::read(buf).map(Fragment::Certificate)
            }
            Some(FragmentTag::UpdateProposal) => {
                SignedUpdateProposal::read(buf).map(Fragment::UpdateProposal)
            }
            Some(FragmentTag::UpdateVote) => SignedUpdateVote::read(buf).map(Fragment::UpdateVote),
            None => Err(ReadError::UnknownTag(tag as u32)),
        }
    }
}

impl property::Serialize for Fragment {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        self.to_raw().serialize(writer)
    }
}

impl property::Deserialize for Fragment {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        let raw = FragmentRaw::deserialize(reader)?;
        Fragment::from_raw(&raw)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
    }
}

impl property::Fragment for Fragment {
    type Id = FragmentId;

    /// The ID of a fragment is a hash of its serialization *without* the size.
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
                0 => Fragment::Initial(Arbitrary::arbitrary(g)),
                1 => Fragment::OldUtxoDeclaration(Arbitrary::arbitrary(g)),
                2 => Fragment::Transaction(Arbitrary::arbitrary(g)),
                3 => Fragment::Certificate(Arbitrary::arbitrary(g)),
                4 => Fragment::UpdateProposal(Arbitrary::arbitrary(g)),
                _ => Fragment::UpdateVote(Arbitrary::arbitrary(g)),
            }
        }
    }
}

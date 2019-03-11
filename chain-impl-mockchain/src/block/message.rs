use chain_core::{packer, property};
use chain_crypto::Ed25519Extended;
use chain_addr::Address;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::mem::size_of;

use crate::{
    certificate,
    key::{Hash, Signed},
    setting,
    transaction::SignedTransaction,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Transaction(SignedTransaction<Address>),

    StakeKeyRegistration(Signed<certificate::StakeKeyRegistration, Ed25519Extended>),
    StakeKeyDeregistration(Signed<certificate::StakeKeyDeregistration, Ed25519Extended>),
    StakeDelegation(Signed<certificate::StakeDelegation, Ed25519Extended>),
    StakePoolRegistration(Signed<certificate::StakePoolRegistration, Ed25519Extended>),
    StakePoolRetirement(Signed<certificate::StakePoolRetirement, Ed25519Extended>),

    // FIXME: Placeholder for the eventual update mechanism. Currently
    // update proposals take effect immediately and there is no
    // signing/voting.
    Update(setting::UpdateProposal),
}

#[derive(FromPrimitive)]
enum MessageTag {
    Transaction = 1,
    StakeKeyRegistration = 2,
    StakeKeyDeregistration = 3,
    StakeDelegation = 4,
    StakePoolRegistration = 5,
    StakePoolRetirement = 6,
    Update = 7,
}

type MessageSize = u16;

fn serialize_buffered<T, W>(
    codec: packer::Codec<W>,
    tag: MessageTag,
    t: &T,
) -> std::io::Result<packer::Codec<W>>
where
    T: property::Serialize<Error = std::io::Error>,
    W: std::io::Write,
{
    let mut buffered = codec.buffered();
    let hole = buffered.hole(size_of::<MessageSize>())?;
    buffered.put_u8(tag as u8)?;
    t.serialize(&mut buffered)?;
    buffered.fill_hole_u16(
        hole,
        (buffered.buffered_len() - size_of::<MessageSize>()) as MessageSize,
    );
    buffered.into_inner()
}

impl property::Serialize for Message {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let codec = Codec::from(writer);
        let _codec = match self {
            Message::Transaction(signed) => {
                serialize_buffered(codec, MessageTag::Transaction, signed)?
            }
            Message::StakeKeyRegistration(signed) => {
                serialize_buffered(codec, MessageTag::StakeKeyRegistration, signed)?
            }
            Message::StakeKeyDeregistration(signed) => {
                serialize_buffered(codec, MessageTag::StakeKeyDeregistration, signed)?
            }
            Message::StakeDelegation(signed) => {
                serialize_buffered(codec, MessageTag::StakeDelegation, signed)?
            }
            Message::StakePoolRegistration(signed) => {
                serialize_buffered(codec, MessageTag::StakePoolRegistration, signed)?
            }
            Message::StakePoolRetirement(signed) => {
                serialize_buffered(codec, MessageTag::StakePoolRetirement, signed)?
            }
            Message::Update(proposal) => serialize_buffered(codec, MessageTag::Update, proposal)?,
        };
        Ok(())
    }
}

impl Message {
    pub(crate) fn deserialize_with_size<R: std::io::BufRead>(
        reader: R,
    ) -> Result<(Self, u16), std::io::Error> {
        use chain_core::packer::*;
        use chain_core::property::Deserialize;
        let mut codec = Codec::from(reader);
        let size = codec.get_u16()? + 2;
        let tag = codec.get_u8()?;
        match MessageTag::from_u8(tag) {
            Some(MessageTag::Transaction) => SignedTransaction::deserialize(&mut codec)
                .map(|msg| (Message::Transaction(msg), size)),
            Some(MessageTag::StakeKeyRegistration) => Signed::deserialize(&mut codec)
                .map(|msg| (Message::StakeKeyRegistration(msg), size)),
            Some(MessageTag::StakeKeyDeregistration) => Signed::deserialize(&mut codec)
                .map(|msg| (Message::StakeKeyDeregistration(msg), size)),
            Some(MessageTag::StakeDelegation) => {
                Signed::deserialize(&mut codec).map(|msg| (Message::StakeDelegation(msg), size))
            }
            Some(MessageTag::StakePoolRegistration) => Signed::deserialize(&mut codec)
                .map(|msg| (Message::StakePoolRegistration(msg), size)),
            Some(MessageTag::StakePoolRetirement) => {
                Signed::deserialize(&mut codec).map(|msg| (Message::StakePoolRetirement(msg), size))
            }
            Some(MessageTag::Update) => setting::UpdateProposal::deserialize(&mut codec)
                .map(|msg| (Message::Update(msg), size)),
            None => panic!("Unrecognized certificate message tag {}.", tag),
        }
    }
}

impl property::Deserialize for Message {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        Ok(Self::deserialize_with_size(reader)?.0)
    }
}

// FIXME: should this be a wrapper type?
pub type MessageId = Hash;

impl property::Message for Message {
    type Id = MessageId;

    /// The ID of a message is a hash of its serialization *without* the size.
    fn id(&self) -> Self::Id {
        use chain_core::property::Serialize;

        // TODO: we should be able to avoid to serialise the whole message
        // in memory, using a hasher.
        let bytes = self
            .serialize_as_vec()
            .expect("In memory serialization is expected to work");
        Hash::hash_bytes(&bytes[size_of::<MessageSize>()..])
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Message {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            match g.next_u32() % 100 {
                0 => Message::StakeKeyRegistration(Arbitrary::arbitrary(g)),
                1 => Message::StakeKeyDeregistration(Arbitrary::arbitrary(g)),
                2 => Message::StakeDelegation(Arbitrary::arbitrary(g)),
                3 => Message::StakePoolRegistration(Arbitrary::arbitrary(g)),
                4 => Message::StakePoolRetirement(Arbitrary::arbitrary(g)),
                _ => Message::Transaction(Arbitrary::arbitrary(g)),
            }
        }
    }
}

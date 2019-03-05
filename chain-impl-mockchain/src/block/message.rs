use chain_core::property;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use crate::{certificate, key::Signed, setting, transaction::SignedTransaction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Transaction(SignedTransaction),

    StakeKeyRegistration(Signed<certificate::StakeKeyRegistration>),
    StakeKeyDeregistration(Signed<certificate::StakeKeyDeregistration>),
    StakeDelegation(Signed<certificate::StakeDelegation>),
    StakePoolRegistration(Signed<certificate::StakePoolRegistration>),
    StakePoolRetirement(Signed<certificate::StakePoolRetirement>),

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

impl property::Serialize for Message {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        match self {
            Message::Transaction(signed) => {
                codec.put_u8(MessageTag::Transaction as u8)?;
                signed.serialize(&mut codec)
            }
            Message::StakeKeyRegistration(signed) => {
                codec.put_u8(MessageTag::StakeKeyRegistration as u8)?;
                signed.serialize(&mut codec)
            }
            Message::StakeKeyDeregistration(signed) => {
                codec.put_u8(MessageTag::StakeKeyDeregistration as u8)?;
                signed.serialize(&mut codec)
            }
            Message::StakeDelegation(signed) => {
                codec.put_u8(MessageTag::StakeDelegation as u8)?;
                signed.serialize(&mut codec)
            }
            Message::StakePoolRegistration(signed) => {
                codec.put_u8(MessageTag::StakePoolRegistration as u8)?;
                signed.serialize(&mut codec)
            }
            Message::StakePoolRetirement(signed) => {
                codec.put_u8(MessageTag::StakePoolRetirement as u8)?;
                signed.serialize(&mut codec)
            }
            Message::Update(proposal) => {
                codec.put_u8(MessageTag::Update as u8)?;
                proposal.serialize(&mut codec)
            }
        }
    }
}

impl property::Deserialize for Message {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(reader);
        let tag = codec.get_u8()?;
        match MessageTag::from_u8(tag) {
            Some(MessageTag::Transaction) => Ok(Message::Transaction(
                SignedTransaction::deserialize(&mut codec)?,
            )),
            Some(MessageTag::StakeKeyRegistration) => Ok(Message::StakeKeyRegistration(
                Signed::deserialize(&mut codec)?,
            )),
            Some(MessageTag::StakeKeyDeregistration) => Ok(Message::StakeKeyDeregistration(
                Signed::deserialize(&mut codec)?,
            )),
            Some(MessageTag::StakeDelegation) => {
                Ok(Message::StakeDelegation(Signed::deserialize(&mut codec)?))
            }
            Some(MessageTag::StakePoolRegistration) => Ok(Message::StakePoolRegistration(
                Signed::deserialize(&mut codec)?,
            )),
            Some(MessageTag::StakePoolRetirement) => Ok(Message::StakePoolRetirement(
                Signed::deserialize(&mut codec)?,
            )),
            Some(MessageTag::Update) => Ok(Message::Update(setting::UpdateProposal::deserialize(
                &mut codec,
            )?)),
            None => panic!("Unrecognized certificate message tag {}.", tag),
        }
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

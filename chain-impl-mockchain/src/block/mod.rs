//! Representation of the block in the mockchain.
use crate::key::{make_signature, make_signature_update, Hash};
use crate::leadership::{bft, genesis::GenesisPraosLeader, Leader};
use chain_core::property::{self, Header as _, Serialize};
use chain_crypto::Verification;

mod builder;
//mod cstruct;
mod header;
pub mod message;
mod version;

pub use self::version::*;

pub use self::builder::BlockBuilder;

pub use self::header::{
    GenesisPraosProof, Header, KESSignature, Proof,
};
pub use self::version::*;

pub use crate::date::{BlockDate, BlockDateParseError};

/// `Block` is an element of the blockchain it contains multiple
/// transaction and a reference to the parent block. Alongside
/// with the position of that block in the chain.
#[derive(Debug, Clone)]
pub struct Block {
    pub header: Header,
    pub contents: BlockContents,
}

impl PartialEq for Block {
    fn eq(&self, rhs: &Self) -> bool {
        self.header.hash() == rhs.header.hash()
    }
}
impl Eq for Block {}

#[derive(Debug, Clone)]
pub struct BlockContents(Vec<Message>);

impl PartialEq for BlockContents {
    fn eq(&self, rhs: &Self) -> bool {
        self.compute_hash_size() == rhs.compute_hash_size()
    }
}
impl Eq for BlockContents {}

impl BlockContents {
    #[inline]
    pub fn new(messages: Vec<Message>) -> Self {
        BlockContents(messages)
    }
    #[inline]
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a Message> {
        self.0.iter()
    }
    pub fn compute_hash_size(&self) -> (BlockContentHash, usize) {
        let mut bytes = Vec::with_capacity(4096);

        for message in self.iter() {
            message.serialize(&mut bytes).unwrap();
        }

        let hash = Hash::hash_bytes(&bytes);
        (hash, bytes.len())
    }
}

impl Block {
    /// Create a new signed block.
    #[deprecated(note = "utilise BlockBuilder instead")]
    pub fn new(contents: BlockContents, common: Common, leader: &mut Leader) -> Self {
        let proof = match leader {
            Leader::None => Proof::None,
            Leader::BftLeader(private_key) => {
                let signature = make_signature(&private_key, &common);
                Proof::Bft(BftProof {
                    leader_id: bft::LeaderId(private_key.to_public()),
                    signature: BftSignature(signature),
                })
            }
            Leader::GenesisPraos(ref mut kes_secret, vrf_secret, proven_output_seed) => {
                let gpleader = GenesisPraosLeader {
                    kes_public_key: kes_secret.to_public(),
                    vrf_public_key: vrf_secret.to_public(),
                };
                let signature = make_signature_update(kes_secret, &common);
                Proof::GenesisPraos(GenesisPraosProof {
                    genesis_praos_id: gpleader.get_id(),
                    vrf_proof: proven_output_seed.clone(),
                    kes_proof: KESSignature(signature),
                    //vrf_public_key: vrf_secret.public(),
                    //kes_public_key: kes_secret.to_public().into(),
                })
            }
        };
        Block {
            header: Header {
                common: common,
                proof: proof,
            },
            contents: contents,
        }
    }

    /// Verify if block is correctly signed by the key.
    /// Return `false` if there is no such signature or
    /// if it can't be verified.
    pub fn verify(&self) -> bool {
        let header_proof = self.header.verify_proof();

        let (content_hash, content_size) = self.contents.compute_hash_size();

        header_proof == Verification::Success
            && &content_hash == self.header.block_content_hash()
            && content_size == self.header.common.block_content_size as usize
    }
}

impl property::Block for Block {
    type Id = BlockId;
    type Date = BlockDate;
    type Version = BlockVersion;
    type ChainLength = ChainLength;

    /// Identifier of the block, currently the hash of the
    /// serialized transaction.
    fn id(&self) -> Self::Id {
        self.header.id()
    }

    /// Id of the parent block.
    fn parent_id(&self) -> Self::Id {
        *self.header.block_parent_hash()
    }

    /// Date of the block.
    fn date(&self) -> Self::Date {
        *self.header.block_date()
    }

    fn version(&self) -> Self::Version {
        *self.header.block_version()
    }

    fn chain_length(&self) -> Self::ChainLength {
        self.header.chain_length()
    }
}

impl property::HasHeader for Block {
    type Header = Header;
    fn header(&self) -> Self::Header {
        self.header.clone()
    }
}

impl property::Serialize for Block {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        self.header.serialize(&mut writer)?;

        for message in self.contents.iter() {
            message.serialize(&mut writer)?;
        }
        Ok(())
    }
}

impl property::Deserialize for Block {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(mut reader: R) -> Result<Self, Self::Error> {
        let header = Header::deserialize(&mut reader)?;

        let mut serialized_content_size = header.common.block_content_size;
        let mut contents = BlockContents(Vec::with_capacity(15_000));
        while serialized_content_size > 0 {
            let (message, message_size) = Message::deserialize_with_size(&mut reader)?;
            contents.0.push(message);
            dbg!(contents.0.len());

            serialized_content_size -= message_size as u32;
            dbg!(serialized_content_size);
        }

        Ok(Block {
            header: header,
            contents: contents,
        })
    }
}

impl property::HasMessages for Block {
    type Message = Message;
    fn messages<'a>(&'a self) -> Box<Iterator<Item = &Message> + 'a> {
        Box::new(self.contents.iter())
    }

    fn for_each_message<F>(&self, mut f: F)
    where
        F: FnMut(&Self::Message),
    {
        self.contents.iter().for_each(|msg| f(msg))
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};

    quickcheck! {
        fn block_serialization_bijection(b: Block) -> TestResult {
            property::testing::serialization_bijection(b)
        }
    }

    impl Arbitrary for BlockContents {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let len = u8::arbitrary(g) % 12;
            BlockContents(
                std::iter::repeat_with(|| Arbitrary::arbitrary(g))
                    .take(len as usize)
                    .collect(),
            )
        }
    }
    impl Arbitrary for Block {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let content = BlockContents::arbitrary(g);
            let (hash, size) = content.compute_hash_size();
            let mut header = Header::arbitrary(g);
            header.common.block_content_size = size as u32;
            header.common.block_content_hash = hash;
            Block {
                header: header,
                contents: content,
            }
        }
    }
}

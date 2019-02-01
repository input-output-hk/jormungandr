//! Representation of the block in the mockchain.
use crate::key::*;
use crate::transaction::*;
use chain_core::property;

/// Non unique identifier of the transaction position in the
/// blockchain. There may be many transactions related to the same
/// `SlotId`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlotId(u64);

impl property::BlockDate for SlotId {}

/// `Block` is an element of the blockchain it contains multiple
/// transaction and a reference to the parent block. Alongside
/// with the position of that block in the chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub slot_id: SlotId,
    pub parent_hash: Hash,

    pub transactions: Vec<SignedTransaction>,
}

/// `Block` is an element of the blockchain it contains multiple
/// transaction and a reference to the parent block. Alongside
/// with the position of that block in the chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedBlock {
    /// Public key used to sign the block.
    pub public_key: PublicKey,
    /// List of cryptographic signatures that verifies the block.
    pub signature: Signature,
    /// Internal block.
    pub block: Block,
}

impl SlotId {
    /// access the block number since the beginning of the blockchain
    pub fn block_number(&self) -> u64 {
        self.0
    }
}

impl SignedBlock {
    /// Create a new signed block.
    pub fn new(block: Block, pkey: &PrivateKey) -> Self {
        use chain_core::property::Block;
        let block_id = block.id();
        SignedBlock {
            public_key: pkey.public(),
            signature: pkey.sign(block_id.as_ref()),
            block: block,
        }
    }

    /// Verify if block is correctly signed by the key.
    /// Return `false` if there is no such signature or
    /// if it can't be verified.
    pub fn verify(&self) -> bool {
        use chain_core::property::Block;
        let block_id = self.block.id();
        self.public_key.verify(block_id.as_ref(), &self.signature)
    }
}

impl property::Block for Block {
    type Id = Hash;
    type Date = SlotId;

    /// Identifier of the block, currently the hash of the
    /// serialized transaction.
    fn id(&self) -> Self::Id {
        use chain_core::property::Serialize;
        // TODO: hash creation can be much faster
        let bytes = self
            .serialize_as_vec()
            .expect("expect serialisation in memory to never fail");
        Hash::hash_bytes(&bytes)
    }

    /// Id of the parent block.
    fn parent_id(&self) -> Self::Id {
        self.parent_hash
    }

    /// Date of the block.
    fn date(&self) -> Self::Date {
        self.slot_id
    }
}
impl property::Block for SignedBlock {
    type Id = <Block as property::Block>::Id;
    type Date = <Block as property::Block>::Date;

    /// Identifier of the block, currently the hash of the
    /// serialized transaction.
    fn id(&self) -> Self::Id {
        self.block.id()
    }

    /// Id of the parent block.
    fn parent_id(&self) -> Self::Id {
        self.block.parent_id()
    }

    /// Date of the block.
    fn date(&self) -> Self::Date {
        self.block.date()
    }
}

impl property::Serialize for Block {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::Codec;
        use std::io::Write;

        let mut codec = Codec::from(writer);

        codec.put_u64(self.slot_id.0)?;
        codec.write_all(self.parent_hash.as_ref())?;
        codec.put_u16(self.transactions.len() as u16)?;
        for t in self.transactions.iter() {
            t.serialize(&mut codec)?;
        }

        Ok(())
    }
}
impl property::Serialize for SignedBlock {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        self.public_key.serialize(&mut writer)?;
        self.signature.serialize(&mut writer)?;
        self.block.serialize(&mut writer)
    }
}

impl property::Deserialize for Block {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::Codec;
        use std::io::Read;

        let mut codec = Codec::from(reader);

        let date = codec.get_u64().map(SlotId)?;

        let mut hash = [0; 32];
        codec.read_exact(&mut hash)?;
        let hash = Hash::from(cardano::hash::Blake2b256::from(hash));

        let num_transactions = codec.get_u16()? as usize;

        let mut block = Block {
            slot_id: date,
            parent_hash: hash,
            transactions: Vec::with_capacity(num_transactions),
        };
        for _ in 0..num_transactions {
            block
                .transactions
                .push(SignedTransaction::deserialize(&mut codec)?);
        }

        Ok(block)
    }
}
impl property::Deserialize for SignedBlock {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(mut reader: R) -> Result<Self, Self::Error> {
        let public_key = PublicKey::deserialize(&mut reader)?;
        let signature = Signature::deserialize(&mut reader)?;
        let block = Block::deserialize(&mut reader)?;

        Ok(SignedBlock {
            public_key,
            signature,
            block,
        })
    }
}

impl property::HasTransaction<SignedTransaction> for Block {
    fn transactions<'a>(&'a self) -> std::slice::Iter<'a, SignedTransaction> {
        self.transactions.iter()
    }
}
impl property::HasTransaction<SignedTransaction> for SignedBlock {
    fn transactions<'a>(&'a self) -> std::slice::Iter<'a, SignedTransaction> {
        self.block.transactions()
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

        fn signed_block_serialization_bijection(b: SignedBlock) -> TestResult {
            property::testing::serialization_bijection(b)
        }
    }

    impl Arbitrary for Block {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Block {
                slot_id: Arbitrary::arbitrary(g),
                parent_hash: Arbitrary::arbitrary(g),
                transactions: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for SignedBlock {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            SignedBlock {
                block: Arbitrary::arbitrary(g),
                public_key: Arbitrary::arbitrary(g),
                signature: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for SlotId {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            SlotId(Arbitrary::arbitrary(g))
        }
    }
}

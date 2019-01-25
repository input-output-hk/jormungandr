//! Representation of the block in the mockchain.
use crate::key::*;
use crate::transaction::*;
use bincode;
use chain_core::property;

/// Non unique identifier of the transaction position in the
/// blockchain. There may be many transactions related to the same
/// `SlotId`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct SlotId(u32, u32);

impl property::BlockDate for SlotId {}

/// `Block` is an element of the blockchain it contains multiple
/// transaction and a reference to the parent block. Alongside
/// with the position of that block in the chain.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Block {
    pub slot_id: SlotId,
    pub parent_hash: Hash,

    pub transactions: Vec<SignedTransaction>,
}

impl property::Block for Block {
    type Id = Hash;
    type Date = SlotId;

    /// Identifier of the block, currently the hash of the
    /// serialized transaction.
    fn id(&self) -> Self::Id {
        let bytes = bincode::serialize(self).expect("unable to serialize block");
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

impl property::Serialize for Block {
    // FIXME: decide on appropriate format for mock blockchain

    type Error = bincode::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        bincode::serialize_into(writer, self)
    }
}

impl property::Deserialize for Block {
    // FIXME: decide on appropriate format for mock blockchain

    type Error = bincode::Error;

    fn deserialize<R: std::io::Read>(reader: R) -> Result<Block, bincode::Error> {
        bincode::deserialize_from(reader)
    }
}

impl property::HasTransaction<SignedTransaction> for Block {
    fn transactions<'a>(&'a self) -> std::slice::Iter<'a, SignedTransaction> {
        self.transactions.iter()
    }
}

/// `Block` is an element of the blockchain it contains multiple
/// transaction and a reference to the parent block. Alongside
/// with the position of that block in the chain.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SignedBlock {
    /// Internal block.
    block: Block,
    /// Public key used to sign the block.
    public_key: PublicKey,
    /// List of cryptographic signatures that verifies the block.
    signature: Signature,
}

impl SignedBlock {
    /// Create a new signed block.
    pub fn new(block: Block, pkey: PrivateKey) -> Self {
        use chain_core::property::Block;
        let block_id = block.id();
        SignedBlock {
            block: block,
            public_key: pkey.public(),
            signature: pkey.sign(block_id.as_ref()),
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

impl property::Serialize for SignedBlock {
    type Error = bincode::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        bincode::serialize_into(writer, self)
    }
}

impl property::Deserialize for SignedBlock {
    type Error = bincode::Error;

    fn deserialize<R: std::io::Read>(reader: R) -> Result<Self, bincode::Error> {
        bincode::deserialize_from(reader)
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

#[cfg(test)]
mod test {

    use super::*;
    use quickcheck::{Arbitrary, Gen};

    quickcheck! {
        fn block_serialization_bijection(b: Block) -> bool {
            property::testing::serialization_bijection(b)
        }

        fn signed_block_serialization_bijection(b: SignedBlock) -> bool {
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
            SlotId(Arbitrary::arbitrary(g), Arbitrary::arbitrary(g))
        }
    }
}

//! Representation of the block in the mockchain.
use crate::key::*;
use crate::transaction::*;
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
    type Header = ();

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

    fn header(&self) -> Self::Header {
        ()
    }
}

impl chain_core::property::Serializable for Block {
    // FIXME: decide on appropriate format for mock blockchain

    type Error = bincode::Error;

    fn deserialize<R: std::io::Read>(reader: R) -> Result<Block, bincode::Error> {
        bincode::deserialize_from(reader)
    }

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        bincode::serialize_into(writer, self)
    }
}

impl property::HasTransaction<SignedTransaction> for Block {
    fn transactions<'a>(&'a self) -> std::slice::Iter<'a, SignedTransaction> {
        self.transactions.iter()
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Block {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Block {
                slot_id: Arbitrary::arbitrary(g),
                parent_hash: Arbitrary::arbitrary(g),
                transactions: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for SlotId {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            SlotId(Arbitrary::arbitrary(g), Arbitrary::arbitrary(g))
        }
    }
}

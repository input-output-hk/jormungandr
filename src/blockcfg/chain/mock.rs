use crate::chain::generic as chain;
use crate::ledger::generic as ledger;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlotId(u32, u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash(u64);
impl Hash {
    pub fn hash<T: std::hash::Hash>(t: T) -> Self {
      use std::collections::hash_map::DefaultHasher;
      use std::hash::{Hasher};
      let mut s = DefaultHasher::new();
      t.hash(&mut s);
      Hash(s.finish())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PublicKey(u64);
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrivateKey(u64);
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Signature(u64);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Value(u64);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Address(Hash);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Input(pub TransactionId, pub u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignedInput {
    pub input: Input,
    pub signature: Signature,
    pub public_key: PublicKey,
}
impl SignedInput {
    pub fn verify(&self, output: &Output) -> bool {
        unimplemented!()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Output(pub Address, pub Value);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransactionId(Hash);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Transaction {
    pub inputs: Vec<SignedInput>,
    pub outputs: Vec<Output>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Block {
    pub slot_id: SlotId,
    pub parent_hash: Hash,

    pub transactions: Vec<Transaction>,
}

impl chain::Block for Block {
    type Hash = Hash;
    type Id = SlotId;

    fn parent_hash(&self) -> &Self::Hash { &self.parent_hash }
    fn slot_id(&self) -> Self::Id { self.slot_id }
}
impl<'a> ledger::HasTransaction<'a> for Block {
    type Transaction = Transaction;
    type TransactionIterator = std::slice::Iter<'a, Self::Transaction>;

    fn transactions(&'a self) -> Self::TransactionIterator
    {
        self.transactions.iter()
    }
}
impl ledger::Transaction for Transaction {
    type Input  = Input;
    type Output = Output;
    type Id = TransactionId;
    fn id(&self) -> Self::Id {
        TransactionId(Hash::hash(self))
    }
}

#[cfg(test)]
use quickcheck::{Arbitrary, Gen};

#[cfg(test)]
impl Arbitrary for SlotId {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        SlotId(
            Arbitrary::arbitrary(g),
            Arbitrary::arbitrary(g)
        )
    }
}
#[cfg(test)]
impl Arbitrary for Hash {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Hash(
            Arbitrary::arbitrary(g)
        )
    }
}
#[cfg(test)]
impl Arbitrary for Value {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Value(
            Arbitrary::arbitrary(g)
        )
    }
}
#[cfg(test)]
impl Arbitrary for Address {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Address(
            Arbitrary::arbitrary(g)
        )
    }
}
#[cfg(test)]
impl Arbitrary for TransactionId {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        TransactionId(
            Arbitrary::arbitrary(g)
        )
    }
}
#[cfg(test)]
impl Arbitrary for Signature {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Signature(Arbitrary::arbitrary(g))
    }
}
#[cfg(test)]
impl Arbitrary for PrivateKey {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        PrivateKey(Arbitrary::arbitrary(g))
    }
}
#[cfg(test)]
impl Arbitrary for PublicKey {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        PublicKey(Arbitrary::arbitrary(g))
    }
}
#[cfg(test)]
impl Arbitrary for Input {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Input(Arbitrary::arbitrary(g),Arbitrary::arbitrary(g))
    }
}
#[cfg(test)]
impl Arbitrary for SignedInput {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        SignedInput {
            input: Arbitrary::arbitrary(g),
            signature: Arbitrary::arbitrary(g),
            public_key: Arbitrary::arbitrary(g),
        }
    }
}
#[cfg(test)]
impl Arbitrary for Output {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Output(Arbitrary::arbitrary(g),Arbitrary::arbitrary(g))
    }
}
#[cfg(test)]
impl Arbitrary for Transaction {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Transaction {
            inputs: Arbitrary::arbitrary(g),
            outputs: Arbitrary::arbitrary(g),
        }
    }
}
#[cfg(test)]
impl Arbitrary for Block {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Block {
            slot_id: Arbitrary::arbitrary(g),
            parent_hash: Arbitrary::arbitrary(g),
            transactions: Arbitrary::arbitrary(g),
        }
    }
}
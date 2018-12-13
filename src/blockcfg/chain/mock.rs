//! This module defines some basic type to try to mock the blockchain
//! and be able to run simpler tests.

use crate::blockcfg::chain;
use crate::blockcfg::ledger;

use cardano::hdwallet as crypto;
use cardano::hash;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlotId(u32, u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash(hash::Blake2b256);
impl Hash {
    pub fn hash_bytes(bytes: &[u8]) -> Self {
        Hash(hash::Blake2b256::new(bytes))
    }
}
impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PublicKey(crypto::XPub);
impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateKey(crypto::XPrv);
impl AsRef<[u8]> for PrivateKey {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature(crypto::Signature<()>);
impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Value(u64);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Address(Hash);
impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Input(pub TransactionId, pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedInput {
    pub input: Input,
    pub signature: Signature,
    pub public_key: PublicKey,
}
impl SignedInput {
    pub fn verify(&self, _output: &Output) -> bool {
        unimplemented!()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Output(pub Address, pub Value);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransactionId(Hash);
impl AsRef<[u8]> for TransactionId {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub inputs: Vec<SignedInput>,
    pub outputs: Vec<Output>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub slot_id: SlotId,
    pub parent_hash: Hash,

    pub transactions: Vec<Transaction>,
}

impl PrivateKey {
    pub fn public(&self) -> PublicKey {
        PublicKey(self.0.public())
    }
    pub fn sign(&self, data: &[u8]) -> Signature {
        Signature(self.0.sign(data))
    }
}
impl PublicKey {
    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        self.0.verify(message, &signature.0)
    }
}
impl chain::Block for Block {
    type Hash = Hash;
    type Id = SlotId;

    fn parent_hash(&self) -> &Self::Hash { &self.parent_hash }
    fn slot_id(&self) -> Self::Id { self.slot_id }
}
impl ledger::HasTransaction for Block {
    type Transaction = Transaction;

    fn transactions<'a>(&'a self) -> std::slice::Iter<'a, Self::Transaction>
    {
        self.transactions.iter()
    }
}
impl ledger::Transaction for Transaction {
    type Input  = Input;
    type Output = Output;
    type Id = TransactionId;
    fn id(&self) -> Self::Id {
        use std::convert::AsRef;
        let mut bytes : Vec<u8> = vec![];
        for signed_input in self.inputs.iter() {
            bytes.extend(signed_input.input.0.as_ref());
            #[cfg(nightly)] // TODO: github's issue #91
            bytes.extend(signed_input.input.1.to_be_bytes().as_ref());
            bytes.extend(signed_input.signature.as_ref());
            bytes.extend(signed_input.public_key.as_ref());
        }
        for output in self.outputs.iter() {
            bytes.extend(output.0.as_ref());
            #[cfg(nightly)] // TODO: github's issue #91
            bytes.extend(output.1.to_be_bytes().as_ref());
        }
        TransactionId(Hash::hash_bytes(&bytes))
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
        let mut bytes = [0u8;16];
        for byte in bytes.iter_mut() {
            *byte = Arbitrary::arbitrary(g);
        }
        Hash(
            hash::Blake2b256::new(&bytes)
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
        let mut signature = [0;crypto::SIGNATURE_SIZE];
        for byte in signature.iter_mut() {
            *byte = Arbitrary::arbitrary(g);
        }
        Signature(
            crypto::Signature::from_bytes(signature)
        )
    }
}
#[cfg(test)]
impl Arbitrary for PrivateKey {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut xprv = [0;crypto::XPRV_SIZE];
        for byte in xprv.iter_mut() {
            *byte = Arbitrary::arbitrary(g);
        }
        PrivateKey(
            crypto::XPrv::normalize_bytes(xprv)
        )
    }
}
#[cfg(test)]
impl Arbitrary for PublicKey {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut xpub = [0;crypto::XPUB_SIZE];
        for byte in xpub.iter_mut() {
            *byte = Arbitrary::arbitrary(g);
        }
        PublicKey(
            crypto::XPub::from_bytes(xpub)
        )
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

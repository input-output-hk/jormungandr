//! This module defines some basic type to try to mock the blockchain
//! and be able to run simpler tests.
//!

use std::collections::HashMap;

use crate::blockcfg::{property, serialization};

use cardano::hash;
use cardano::hdwallet as crypto;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct SlotId(u32, u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct Hash(hash::Blake2b256);
impl Hash {
    pub fn hash_bytes(bytes: &[u8]) -> Self {
        Hash(hash::Blake2b256::new(bytes))
    }
}
impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

/// TODO: this public key contains the chain code in it too
/// during serialisation this might not be needed
/// removing it will save 32bytes of non necessary storage (github #93)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct PublicKey(crypto::XPub);
impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateKey(crypto::XPrv);
impl AsRef<[u8]> for PrivateKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Signature(crypto::Signature<()>);
impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct Value(u64);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct Address(Hash);
impl Address {
    pub fn new(public_key: &PublicKey) -> Self {
        Address(Hash::hash_bytes(public_key.as_ref()))
    }
}
impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct Input {
    pub transaction_id: TransactionId,
    pub output_index: u32,
}
impl Input {
    pub fn new(transaction_id: TransactionId, output_index: u32) -> Self {
        Input {
            transaction_id,
            output_index,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct Output(pub Address, pub Value);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct TransactionId(Hash);
impl AsRef<[u8]> for TransactionId {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Transaction {
    pub inputs: Vec<SignedInput>,
    pub outputs: Vec<Output>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
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

impl serialization::Deserialize for Block {
    // FIXME: decide on appropriate format for mock blockchain

    type Error = crate::serde_yaml::Error;

    fn deserialize(data: &[u8]) -> Result<Block, crate::serde_yaml::Error> {
        serde_yaml::from_slice(data)
    }
}

impl property::Block for Block {
    type Id = Hash;
    type Date = SlotId;

    fn id(&self) -> Self::Id {
        unimplemented!()
    }
    fn parent_id(&self) -> &Self::Id {
        &self.parent_hash
    }
    fn date(&self) -> Self::Date {
        self.slot_id
    }
}
impl property::HasTransaction for Block {
    type Transaction = Transaction;

    fn transactions<'a>(&'a self) -> std::slice::Iter<'a, Self::Transaction> {
        self.transactions.iter()
    }
}

impl property::Transaction for Transaction {
    type Input = Input;
    type Output = Output;
    type Id = TransactionId;
    fn id(&self) -> Self::Id {
        use std::convert::AsRef;
        let mut bytes: Vec<u8> = vec![];
        for signed_input in self.inputs.iter() {
            bytes.extend(signed_input.input.transaction_id.as_ref());
            #[cfg(nightly)] // TODO: github's issue #91
            bytes.extend(signed_input.input.output_index.to_be_bytes().as_ref());
            bytes.extend(signed_input.signature.as_ref());
            // remove the chain code from the serialisation
            // see github #93
            bytes.extend(signed_input.public_key.as_ref()[..32].as_ref());
        }
        for output in self.outputs.iter() {
            bytes.extend(output.0.as_ref());
            #[cfg(nightly)] // TODO: github's issue #91
            bytes.extend(output.1.to_be_bytes().as_ref());
        }
        TransactionId(Hash::hash_bytes(&bytes))
    }
}

#[derive(Debug, Clone)]
pub struct Ledger {
    unspent_outputs: HashMap<Input, Output>,
}
impl Ledger {
    pub fn new() -> Self {
        Ledger {
            unspent_outputs: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Diff {
    spent_outputs: HashMap<Input, Output>,
    new_unspent_outputs: HashMap<Input, Output>,
}
impl Diff {
    fn new() -> Self {
        Diff {
            spent_outputs: HashMap::new(),
            new_unspent_outputs: HashMap::new(),
        }
    }

    fn extend(&mut self, other: Self) {
        self.new_unspent_outputs.extend(other.new_unspent_outputs);
        self.spent_outputs.extend(other.spent_outputs);
    }
}

#[derive(Debug, Clone)]
pub enum Error {
    /// If the Ledger could not find the given input in the UTxO list it will
    /// report this error.
    InputDoesNotResolve(Input),

    /// if the Ledger finds that the input has already been used once in a given
    /// transaction or block of transactions it will report this error.
    ///
    /// the input here is the given input used twice,
    /// the output here is the output set in the first occurrence of the input, it
    /// will provide a bit of information to the user to figure out what went wrong
    DoubleSpend(Input, Output),

    /// This error will happen if the input was already set and is now replaced
    /// by another output.
    ///
    /// I.E: the value output has changed but the input is the same. This should not
    /// happen since changing the output will change the transaction identifier
    /// associated to this output.
    ///
    /// first the input in common, then the original output and finally the new output
    InputWasAlreadySet(Input, Output, Output),

    /// error occurs if the signature is invalid: either does not match the initial output
    /// or it is not cryptographically valid.
    InvalidSignature(Input, Output, Signature),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::InputDoesNotResolve(_) => write!(f, "Input does not resolve to an UTxO"),
            Error::DoubleSpend(_, _) => write!(f, "UTxO spent twice in the same transaction"),
            Error::InputWasAlreadySet(_, _, _) => {
                write!(f, "Input was already present in the Ledger")
            }
            Error::InvalidSignature(_, _, _) => write!(f, "Input is not signed properly"),
        }
    }
}
impl std::error::Error for Error {}

impl property::Ledger for Ledger {
    type Transaction = Transaction;
    type Diff = Diff;
    type Error = Error;

    fn diff_transaction(&self, transaction: &Self::Transaction) -> Result<Self::Diff, Self::Error> {
        use crate::blockcfg::property::Transaction;

        let mut diff = Diff::new();
        let id = transaction.id();

        // 1. validate the inputs
        for input in transaction.inputs.iter() {
            if let Some(output) = self.unspent_outputs.get(&input.input) {
                if !input.verify(&output) {
                    return Err(Error::InvalidSignature(
                        input.input,
                        *output,
                        input.signature.clone(),
                    ));
                }
                if let Some(output) = diff.spent_outputs.insert(input.input, *output) {
                    return Err(Error::DoubleSpend(input.input, output));
                }
            } else {
                return Err(Error::InputDoesNotResolve(input.input));
            }
        }

        // 2. prepare to add the new outputs
        for (index, output) in transaction.outputs.iter().enumerate() {
            diff.new_unspent_outputs
                .insert(Input::new(id, index as u32), *output);
        }

        Ok(diff)
    }

    fn diff<'a, I>(&self, transactions: I) -> Result<Self::Diff, Self::Error>
    where
        I: Iterator<Item = &'a Self::Transaction> + Sized,
        Self::Transaction: 'a,
    {
        let mut diff = Diff::new();

        for transaction in transactions {
            diff.extend(self.diff_transaction(transaction)?);
        }

        Ok(diff)
    }

    fn add(&mut self, diff: Self::Diff) -> Result<&mut Self, Self::Error> {
        for spent_output in diff.spent_outputs.keys() {
            if let None = self.unspent_outputs.remove(spent_output) {
                return Err(Error::InputDoesNotResolve(*spent_output));
            }
        }

        for (input, output) in diff.new_unspent_outputs {
            if let Some(original_output) = self.unspent_outputs.insert(input, output) {
                return Err(Error::InputWasAlreadySet(input, original_output, output));
            }
        }

        Ok(self)
    }
}

#[cfg(test)]
mod quickcheck {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for SlotId {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            SlotId(Arbitrary::arbitrary(g), Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for Hash {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut bytes = [0u8; 16];
            for byte in bytes.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            Hash(hash::Blake2b256::new(&bytes))
        }
    }

    impl Arbitrary for Value {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Value(Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for Address {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Address(Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for TransactionId {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            TransactionId(Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for Signature {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut signature = [0; crypto::SIGNATURE_SIZE];
            for byte in signature.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            Signature(crypto::Signature::from_bytes(signature))
        }
    }

    impl Arbitrary for PrivateKey {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut xprv = [0; crypto::XPRV_SIZE];
            for byte in xprv.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            PrivateKey(crypto::XPrv::normalize_bytes(xprv))
        }
    }

    impl Arbitrary for PublicKey {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut xpub = [0; crypto::XPUB_SIZE];
            for byte in xpub.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            PublicKey(crypto::XPub::from_bytes(xpub))
        }
    }

    impl Arbitrary for Input {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Input {
                transaction_id: Arbitrary::arbitrary(g),
                output_index: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for SignedInput {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            SignedInput {
                input: Arbitrary::arbitrary(g),
                signature: Arbitrary::arbitrary(g),
                public_key: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for Output {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Output(Arbitrary::arbitrary(g), Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for Transaction {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Transaction {
                inputs: Arbitrary::arbitrary(g),
                outputs: Arbitrary::arbitrary(g),
            }
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
}

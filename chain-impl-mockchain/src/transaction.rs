use crate::address::*;
use crate::key::*;
use chain_core::property;

/// Unspent transaction value.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct Value(pub u64);

/// Unspent transaction pointer.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct UtxoPointer {
    pub transaction_id: TransactionId,
    pub output_index: u32,
}
impl UtxoPointer {
    pub fn new(transaction_id: TransactionId, output_index: u32) -> Self {
        UtxoPointer {
            transaction_id,
            output_index,
        }
    }
}

/// Structure that proofs that certain user agrees with
/// some data. This structure is used to sign `Transaction`
/// and get `SignedTransaction` out.
///
/// It's important that witness works with opaque structures
/// and may not know the contents of the internal transaction.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Witness {
    pub signature: Signature,
    pub public_key: PublicKey,
}

impl Witness {
    /// Creates new `Witness` value.
    pub fn new(transaction_id: TransactionId, private_key: &PrivateKey) -> Self {
        let sig = private_key.sign(transaction_id.as_ref());
        Witness {
            signature: sig,
            public_key: private_key.public(),
        }
    }

    /// Checks if a witness emitter matches the `Output` address.
    ///
    /// This check is needed because each Utxo in the transaction
    /// must be signed by the wallet holder.
    pub fn matches(&self, output: &Output) -> bool {
        let addr = Address::new(&self.public_key);
        addr == output.0
    }

    /// Verify the given `TransactionId` using the witness.
    pub fn verifies(&self, transaction_id: TransactionId) -> bool {
        self.public_key
            .verify(transaction_id.as_ref(), &self.signature)
    }
}

/// Information how tokens are spent.
/// A value of tokens is sent to the address.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct Output(pub Address, pub Value);

/// Id of the transaction.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct TransactionId(pub Hash);
impl AsRef<[u8]> for TransactionId {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl property::TransactionId for TransactionId {}

/// Transaction, transaction maps old unspent tokens into the
/// set of the new addresses.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Transaction {
    pub inputs: Vec<UtxoPointer>,
    pub outputs: Vec<Output>,
}

impl property::Transaction for Transaction {
    type Input = UtxoPointer;
    type Output = Output;
    type Id = TransactionId;
    fn inputs<'a>(&'a self) -> std::slice::Iter<'a, Self::Input> {
        self.inputs.iter()
    }
    fn outputs<'a>(&'a self) -> std::slice::Iter<'a, Self::Output> {
        self.outputs.iter()
    }
    fn id(&self) -> Self::Id {
        let bytes = bincode::serialize(self).expect("unable to serialize transaction");
        TransactionId(Hash::hash_bytes(&bytes))
    }
}

impl property::Serialize for Transaction {
    type Error = bincode::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), bincode::Error> {
        bincode::serialize_into(writer, self)
    }
}

impl property::Deserialize for Transaction {
    type Error = bincode::Error;

    fn deserialize<R: std::io::Read>(reader: R) -> Result<Transaction, bincode::Error> {
        bincode::deserialize_from(reader)
    }
}

/// Each transaction must be signed in order to be executed
/// by the ledger. `SignedTransaction` represents such a transaction.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SignedTransaction {
    pub tx: Transaction,
    pub witnesses: Vec<Witness>,
}

impl property::Serialize for SignedTransaction {
    type Error = bincode::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        bincode::serialize_into(writer, self)
    }
}

impl property::Deserialize for SignedTransaction {
    type Error = bincode::Error;

    fn deserialize<R: std::io::Read>(reader: R) -> Result<SignedTransaction, bincode::Error> {
        bincode::deserialize_from(reader)
    }
}

impl property::Transaction for SignedTransaction {
    type Input = UtxoPointer;
    type Output = Output;
    type Id = TransactionId;
    fn inputs<'a>(&'a self) -> std::slice::Iter<'a, Self::Input> {
        self.tx.inputs()
    }
    fn outputs<'a>(&'a self) -> std::slice::Iter<'a, Self::Output> {
        self.tx.outputs()
    }
    fn id(&self) -> Self::Id {
        self.tx.id()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use cardano::hdwallet as crypto;

    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Value {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Value(Arbitrary::arbitrary(g))
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

    impl Arbitrary for UtxoPointer {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            UtxoPointer {
                transaction_id: Arbitrary::arbitrary(g),
                output_index: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for Witness {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Witness {
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

    impl Arbitrary for SignedTransaction {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            SignedTransaction {
                tx: Arbitrary::arbitrary(g),
                witnesses: Arbitrary::arbitrary(g),
            }
        }
    }
}

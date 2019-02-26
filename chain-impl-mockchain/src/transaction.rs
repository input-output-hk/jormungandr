use crate::key::*;
use chain_addr::Address;
use chain_core::property;

/// Unspent transaction value.
#[cfg_attr(feature = "generic-serialization", derive(serde_derive::Serialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Value(pub u64);

/// Unspent transaction pointer.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtxoPointer {
    /// the transaction identifier where the unspent output is
    pub transaction_id: TransactionId,
    /// the output index within the pointed transaction's outputs
    pub output_index: u32,
    /// the value we expect to read from this output
    ///
    /// This setting is added in order to protect undesired withdrawal
    /// and to set the actual fee in the transaction.
    pub value: Value,
}
impl UtxoPointer {
    pub fn new(transaction_id: TransactionId, output_index: u32, value: Value) -> Self {
        UtxoPointer {
            transaction_id,
            output_index,
            value,
        }
    }
}

/// Structure that proofs that certain user agrees with
/// some data. This structure is used to sign `Transaction`
/// and get `SignedTransaction` out.
///
/// It's important that witness works with opaque structures
/// and may not know the contents of the internal transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Witness(Signature);

impl Witness {
    /// Creates new `Witness` value.
    pub fn new(transaction_id: &TransactionId, private_key: &PrivateKey) -> Self {
        let sig = private_key.sign(transaction_id.as_ref());
        Witness(sig)
    }

    /// Verify the given `TransactionId` using the witness.
    pub fn verifies(&self, public_key: &PublicKey, transaction_id: &TransactionId) -> bool {
        public_key.verify(transaction_id.as_ref(), &self.0)
    }
}

/// Information how tokens are spent.
/// A value of tokens is sent to the address.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Output(pub Address, pub Value);

pub type TransactionId = Hash;

/// Transaction, transaction maps old unspent tokens into the
/// set of the new addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub inputs: Vec<UtxoPointer>,
    pub outputs: Vec<Output>,
}

/// Each transaction must be signed in order to be executed
/// by the ledger. `SignedTransaction` represents such a transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub witnesses: Vec<Witness>,
}

impl property::Serialize for Value {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        codec.put_u64(self.0)
    }
}

impl property::Serialize for Witness {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        self.0.serialize(writer)
    }
}

impl property::Serialize for Transaction {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;

        let mut codec = Codec::from(writer);

        // store the number of inputs and outputs
        codec.put_u8(self.inputs.len() as u8)?;
        codec.put_u8(self.outputs.len() as u8)?;

        for input in self.inputs.iter() {
            input.transaction_id.serialize(&mut codec)?;
            codec.put_u32(input.output_index)?;
            input.value.serialize(&mut codec)?;
        }
        for output in self.outputs.iter() {
            output.0.serialize(&mut codec)?;
            output.1.serialize(&mut codec)?;
        }
        Ok(())
    }
}

impl property::Serialize for SignedTransaction {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;

        let mut codec = Codec::from(writer);
        codec.put_u8(0x01)?;

        assert_eq!(self.transaction.inputs.len(), self.witnesses.len());

        // encode the transaction body
        self.transaction.serialize(&mut codec)?;

        // encode the signatures
        for witness in self.witnesses.iter() {
            witness.serialize(&mut codec)?;
        }
        Ok(())
    }
}

impl property::Deserialize for Value {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(reader);
        codec.get_u64().map(Value)
    }
}

impl property::Deserialize for Witness {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        Signature::deserialize(reader).map(Witness)
    }
}

impl property::Deserialize for Transaction {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;

        let mut codec = Codec::from(reader);

        let num_inputs = codec.get_u8()? as usize;
        let num_outputs = codec.get_u8()? as usize;

        let mut transaction = Transaction {
            inputs: Vec::with_capacity(num_inputs),
            outputs: Vec::with_capacity(num_outputs),
        };

        for _ in 0..num_inputs {
            let transaction_id = TransactionId::deserialize(&mut codec)?;
            let output_index = codec.get_u32()?;
            let value = Value::deserialize(&mut codec)?;
            transaction.inputs.push(UtxoPointer {
                transaction_id,
                output_index,
                value,
            });
        }

        for _ in 0..num_outputs {
            let address = Address::deserialize(&mut codec)?;
            let value = Value::deserialize(&mut codec)?;
            transaction.outputs.push(Output(address, value));
        }

        Ok(transaction)
    }
}
impl property::Deserialize for SignedTransaction {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;

        let mut codec = Codec::from(reader);

        let _transaction_type = codec.get_u8()?;
        let transaction = Transaction::deserialize(&mut codec)?;
        let num_witnesses = transaction.inputs.len();

        let mut signed_transaction = SignedTransaction {
            transaction: transaction,
            witnesses: Vec::with_capacity(num_witnesses),
        };

        for _ in 0..num_witnesses {
            let witness = Witness::deserialize(&mut codec)?;
            signed_transaction.witnesses.push(witness);
        }

        Ok(signed_transaction)
    }
}

impl property::Transaction for Transaction {
    type Input = UtxoPointer;
    type Output = Output;
    type Inputs = [UtxoPointer];
    type Outputs = [Output];
    type Id = TransactionId;

    fn inputs(&self) -> &Self::Inputs {
        &self.inputs
    }
    fn outputs(&self) -> &Self::Outputs {
        &self.outputs
    }
    fn id(&self) -> Self::Id {
        use chain_core::property::Serialize;

        // TODO: we should be able to avoid to serialise the whole transaction
        // in memory, using a hasher.
        let bytes = self
            .serialize_as_vec()
            .expect("In memory serialization is expected to work");
        Hash::hash_bytes(&bytes)
    }
}

impl property::Transaction for SignedTransaction {
    type Input = UtxoPointer;
    type Output = Output;
    type Inputs = [UtxoPointer];
    type Outputs = [Output];
    type Id = TransactionId;

    fn inputs(&self) -> &Self::Inputs {
        self.transaction.inputs()
    }
    fn outputs(&self) -> &Self::Outputs {
        self.transaction.outputs()
    }
    fn id(&self) -> Self::Id {
        self.transaction.id()
    }
}

impl From<u32> for Value {
    fn from(v: u32) -> Self {
        Value(v as u64)
    }
}

impl AsRef<u64> for Value {
    fn as_ref(&self) -> &u64 {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use cardano::redeem as crypto;

    use quickcheck::{Arbitrary, Gen, TestResult};

    quickcheck! {

        /// ```
        /// \forall w=Witness(tx) => w.verifies(tx)
        /// ```
        fn prop_witness_verifies_own_tx(pk: PrivateKey, tx:TransactionId) -> bool {
            let witness = Witness::new(&tx, &pk);
            witness.verifies(&pk.public(), &tx)
        }

        /// ```
        /// \forall w1,w2:Witness, w1.verifies(t1), w2.verifies(t2):
        ///   w1.verifies(v2) <=> w1 == w2
        /// ```
        fn witness_verifies_only_own_tx(pk: PrivateKey, tx1: TransactionId, tx2: TransactionId) -> bool {
            let witness1 = Witness::new(&tx1, &pk);
            let witness2 = Witness::new(&tx2, &pk);
            (witness1.verifies(&pk.public(), &tx2) && witness1 == witness2)
                || (!witness1.verifies(&pk.public(), &tx2))
        }

        fn transaction_id_is_unique(tx1: Transaction, tx2: Transaction) -> bool {
            chain_core::property::testing::transaction_id_is_unique(tx1, tx2)
        }

        fn transaction_encode_decode(transaction: Transaction) -> TestResult {
            chain_core::property::testing::serialization_bijection(transaction)
        }
        fn signed_transaction_encode_decode(transaction: SignedTransaction) -> TestResult {
            chain_core::property::testing::serialization_bijection(transaction)
        }
    }

    impl Arbitrary for Value {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Value(Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for Signature {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut signature = [0; crypto::SIGNATURE_SIZE];
            for byte in signature.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            crypto::Signature::from_bytes(signature).into()
        }
    }

    impl Arbitrary for UtxoPointer {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            UtxoPointer {
                transaction_id: Arbitrary::arbitrary(g),
                output_index: Arbitrary::arbitrary(g),
                value: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for Witness {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Witness(Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for Output {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Output(Arbitrary::arbitrary(g), Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for Transaction {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let num_inputs = u8::arbitrary(g) as usize;
            let num_outputs = u8::arbitrary(g) as usize;
            Transaction {
                inputs: std::iter::repeat_with(|| Arbitrary::arbitrary(g))
                    .take(num_inputs)
                    .collect(),
                outputs: std::iter::repeat_with(|| Arbitrary::arbitrary(g))
                    .take(num_outputs)
                    .collect(),
            }
        }
    }

    impl Arbitrary for SignedTransaction {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let transaction = Transaction::arbitrary(g);
            let num_witnesses = transaction.inputs.len();
            SignedTransaction {
                transaction: transaction,
                witnesses: std::iter::repeat_with(|| Arbitrary::arbitrary(g))
                    .take(num_witnesses)
                    .collect(),
            }
        }
    }
}

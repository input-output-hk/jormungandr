mod transfer;
mod utxo;

use crate::key::{
    deserialize_signature, serialize_signature, Hash, SpendingPublicKey, SpendingSecretKey,
    SpendingSignature,
};
use crate::value::*;
use chain_addr::Address;
use chain_core::property;
use chain_crypto::Verification;

// to remove..
pub use transfer::*;
pub use utxo::*;

/// Transaction, transaction maps old unspent tokens into the
/// set of the new addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction<OutAddress> {
    pub inputs: Vec<UtxoPointer>,
    pub outputs: Vec<Output<OutAddress>>,
}

/// Each transaction must be signed in order to be executed
/// by the ledger. `SignedTransaction` represents such a transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedTransaction<OutAddress> {
    pub transaction: Transaction<OutAddress>,
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

impl property::Serialize for Transaction<Address> {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;

        let mut codec = Codec::from(writer);

        // store the number of inputs and outputs
        codec.put_u8(self.inputs.len() as u8)?;
        codec.put_u8(self.outputs.len() as u8)?;

        for input in self.inputs.iter() {
            codec.put_u8(input.output_index)?;
            input.transaction_id.serialize(&mut codec)?;
            input.value.serialize(&mut codec)?;
        }
        for output in self.outputs.iter() {
            output.0.serialize(&mut codec)?;
            output.1.serialize(&mut codec)?;
        }
        Ok(())
    }
}

impl property::Serialize for SignedTransaction<Address> {
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

impl property::Deserialize for Transaction<Address> {
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
            let output_index = codec.get_u8()?;
            let transaction_id = TransactionId::deserialize(&mut codec)?;
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
impl property::Deserialize for SignedTransaction<Address> {
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

impl<OutAddress> property::Transaction for Transaction<OutAddress>
where
    Transaction<OutAddress>: property::Serialize + property::Deserialize,
{
    type Input = UtxoPointer;
    type Output = Output<OutAddress>;
    type Inputs = [Self::Input];
    type Outputs = [Self::Output];
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

impl<OutAddress> property::Transaction for SignedTransaction<OutAddress>
where
    Transaction<OutAddress>: property::Transaction,
    SignedTransaction<OutAddress>: property::Serialize + property::Deserialize,
{
    type Input = <Transaction<OutAddress> as property::Transaction>::Input;
    type Output = <Transaction<OutAddress> as property::Transaction>::Output;
    type Inputs = <Transaction<OutAddress> as property::Transaction>::Inputs;
    type Outputs = <Transaction<OutAddress> as property::Transaction>::Outputs;
    type Id = <Transaction<OutAddress> as property::Transaction>::Id;

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

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};

    quickcheck! {
        fn transaction_id_is_unique(tx1: Transaction<Address>, tx2: Transaction<Address>) -> bool {
            chain_core::property::testing::transaction_id_is_unique(tx1, tx2)
        }

        fn transaction_encode_decode(transaction: Transaction<Address>) -> TestResult {
            chain_core::property::testing::serialization_bijection(transaction)
        }
        fn signed_transaction_encode_decode(transaction: SignedTransaction<Address>) -> TestResult {
            chain_core::property::testing::serialization_bijection(transaction)
        }
    }

    impl Arbitrary for Value {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Value(Arbitrary::arbitrary(g))
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

    impl Arbitrary for Output<Address> {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Output(Arbitrary::arbitrary(g), Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for Transaction<Address> {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let num_inputs = u8::arbitrary(g) as usize;
            let num_outputs = u8::arbitrary(g) as usize;
            Transaction {
                inputs: std::iter::repeat_with(|| Arbitrary::arbitrary(g))
                    .take(num_inputs % 8)
                    .collect(),
                outputs: std::iter::repeat_with(|| Arbitrary::arbitrary(g))
                    .take(num_outputs % 8)
                    .collect(),
            }
        }
    }

    impl Arbitrary for SignedTransaction<Address> {
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

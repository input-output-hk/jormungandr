mod transaction;
mod transfer;
mod utxo;
mod witness;

use chain_addr::Address;
use chain_core::mempack::{read_vec, ReadBuf, ReadError, Readable};
use chain_core::property;

// to remove..
pub use transaction::*;
pub use transfer::*;
pub use utxo::*;
pub use witness::*;

/// Each transaction must be signed in order to be executed
/// by the ledger. `SignedTransaction` represents such a transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedTransaction<OutAddress, Extra> {
    pub transaction: Transaction<OutAddress, Extra>,
    pub witnesses: Vec<Witness>,
}

impl<Extra: property::Serialize> property::Serialize for AuthenticatedTransaction<Address, Extra> {
    type Error = Extra::Error;

    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Extra::Error> {
        // encode the transaction body
        self.transaction.serialize(&mut writer)?;

        // encode the signatures
        for witness in self.witnesses.iter() {
            witness.serialize(&mut writer)?;
        }
        Ok(())
    }
}

impl<Extra: Readable> Readable for AuthenticatedTransaction<Address, Extra> {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let transaction = Transaction::read(buf)?;
        let num_witnesses = transaction.inputs.len();
        let witnesses = read_vec(buf, num_witnesses)?;

        let signed_transaction = AuthenticatedTransaction {
            transaction,
            witnesses,
        };

        Ok(signed_transaction)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::certificate::OwnerStakeDelegation;
    use quickcheck::{Arbitrary, Gen, TestResult};

    quickcheck! {
        fn transaction_encode_decode(transaction: Transaction<Address, NoExtra>) -> TestResult {
            chain_core::property::testing::serialization_bijection_r(transaction)
        }
        fn stake_owner_delegation_tx_encode_decode(transaction: Transaction<Address, OwnerStakeDelegation>) -> TestResult {
            chain_core::property::testing::serialization_bijection_r(transaction)
        }
        /*
        fn certificate_tx_encode_decode(transaction: Transaction<Address, Certificate>) -> TestResult {
            chain_core::property::testing::serialization_bijection_r(transaction)
        }
        */
        fn signed_transaction_encode_decode(transaction: AuthenticatedTransaction<Address, NoExtra>) -> TestResult {
            chain_core::property::testing::serialization_bijection_r(transaction)
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

    impl Arbitrary for Input {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Input::from_utxo(Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for NoExtra {
        fn arbitrary<G: Gen>(_: &mut G) -> Self {
            Self
        }
    }

    impl<Extra: Arbitrary> Arbitrary for Transaction<Address, Extra> {
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
                extra: Arbitrary::arbitrary(g),
            }
        }
    }

    impl<Extra: Arbitrary> Arbitrary for AuthenticatedTransaction<Address, Extra> {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let transaction = Transaction::arbitrary(g);
            let num_witnesses = transaction.inputs.len();
            AuthenticatedTransaction {
                transaction: transaction,
                witnesses: std::iter::repeat_with(|| Arbitrary::arbitrary(g))
                    .take(num_witnesses)
                    .collect(),
            }
        }
    }
}

mod builder;
mod element;
mod input;
mod io;
mod payload;
mod transaction;
mod transfer;
mod utxo;
mod witness;

use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property;

// to remove..
pub use builder::TxBuilder;
pub use element::*;
pub use input::*;
pub use io::{Error, InputOutput, InputOutputBuilder, OutputPolicy};
pub use payload::{NoExtra, Payload};
pub use transaction::*;
pub use transfer::*;
pub use utxo::*;
pub use witness::*;

impl<Extra: Payload> property::Serialize for Transaction<Extra> {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        writer.write_all(self.as_ref())
    }
}

impl<Extra: Payload> Readable for Transaction<Extra> {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let utx = UnverifiedTransactionSlice::from(buf.get_slice_end());
        match utx.check() {
            Ok(tx) => Ok(tx.into_owned()),
            Err(_) => Err(ReadError::StructureInvalid("transaction".to_string())),
        }
    }
}

// TEMPORARY
pub type AuthenticatedTransaction<P> = Transaction<P>;

#[cfg(test)]
mod test {
    use super::element::TransactionBindingSignature;
    use super::*;
    use crate::certificate::OwnerStakeDelegation;
    use chain_crypto::VerificationAlgorithm;
    use quickcheck::{Arbitrary, Gen, TestResult};
    use quickcheck_macros::quickcheck;

    quickcheck! {
        fn transaction_encode_decode(transaction: Transaction<NoExtra>) -> TestResult {
            chain_core::property::testing::serialization_bijection_r(transaction)
        }
        fn stake_owner_delegation_tx_encode_decode(transaction: Transaction<OwnerStakeDelegation>) -> TestResult {
            chain_core::property::testing::serialization_bijection_r(transaction)
        }
        /*
        fn certificate_tx_encode_decode(transaction: Transaction<Address, Certificate>) -> TestResult {
            chain_core::property::testing::serialization_bijection_r(transaction)
        }
        */
        fn signed_transaction_encode_decode(transaction: Transaction<NoExtra>) -> TestResult {
            chain_core::property::testing::serialization_bijection_r(transaction)
        }
    }

    use std::fmt::Display;

    fn check_eq<X: Eq + Display>(s1: &str, x1: X, s2: &str, x2: X, s: &str) -> Result<(), String> {
        if x1 == x2 {
            Ok(())
        } else {
            Err(format!(
                "{} and {} have different number of {} : {} != {}",
                s1, s2, x1, x2, s
            ))
        }
    }

    #[quickcheck]
    pub fn check_transaction_accessor_consistent(tx: Transaction<NoExtra>) -> TestResult {
        let slice = tx.as_slice();
        let res = check_eq(
            "tx",
            tx.nb_inputs(),
            "tx-slice",
            slice.nb_inputs(),
            "inputs",
        )
        .and_then(|()| {
            check_eq(
                "tx",
                tx.nb_inputs(),
                "tx-inputs-slice",
                slice.inputs().nb_inputs(),
                "inputs",
            )
        })
        .and_then(|()| {
            check_eq(
                "tx",
                tx.nb_inputs() as usize,
                "tx-inputs-slice-iter",
                slice.inputs().iter().count(),
                "inputs",
            )
        })
        .and_then(|()| {
            check_eq(
                "tx",
                tx.nb_outputs(),
                "tx-outputs-slice",
                slice.outputs().nb_outputs(),
                "outputs",
            )
        })
        .and_then(|()| {
            check_eq(
                "tx",
                tx.nb_outputs() as usize,
                "tx-outputs-slice-iter",
                slice.outputs().iter().count(),
                "outputs",
            )
        })
        .and_then(|()| {
            check_eq(
                "tx",
                tx.nb_witnesses(),
                "tx-witness-slice",
                slice.witnesses().nb_witnesses(),
                "witnesses",
            )
        })
        .and_then(|()| {
            check_eq(
                "tx",
                tx.nb_witnesses() as usize,
                "tx-witness-slice-iter",
                slice.witnesses().iter().count(),
                "witnesses",
            )
        });
        match res {
            Ok(()) => TestResult::passed(),
            Err(e) => TestResult::error(e),
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

    impl<Extra: Arbitrary + Payload> Arbitrary for Transaction<Extra>
    where
        Extra::Auth: Arbitrary,
    {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let payload: Extra = Arbitrary::arbitrary(g);
            let payload_auth: Extra::Auth = Arbitrary::arbitrary(g);

            let num_inputs = u8::arbitrary(g) as usize;
            let num_outputs = u8::arbitrary(g) as usize;

            let inputs: Vec<_> = std::iter::repeat_with(|| Arbitrary::arbitrary(g))
                .take(num_inputs % 16)
                .collect();
            let outputs: Vec<_> = std::iter::repeat_with(|| Arbitrary::arbitrary(g))
                .take(num_outputs % 16)
                .collect();
            let witnesses: Vec<_> = std::iter::repeat_with(|| Arbitrary::arbitrary(g))
                .take(num_inputs % 16)
                .collect();

            TxBuilder::new()
                .set_payload(&payload)
                .set_ios(&inputs, &outputs)
                .set_witnesses(&witnesses)
                .set_payload_auth(&payload_auth)
        }
    }

    impl<A: VerificationAlgorithm> Arbitrary for TransactionBindingSignature<A>
    where
        <A as VerificationAlgorithm>::Signature: std::marker::Send,
        A: 'static,
    {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            TransactionBindingSignature(Arbitrary::arbitrary(g))
        }
    }
}

//! Mockchain ledger. Ledger exists in order to update the
//! current state and verify transactions.

use crate::error::*;
use crate::transaction::*;
use crate::update::TransactionsDiff;
use crate::value::*;
use chain_addr::Address;
use chain_core::property;
use std::collections::HashMap;

/// Basic ledger structure. Ledger is represented as the
/// state of unspent output values, associated with their
/// owner.
#[derive(Debug, Clone)]
pub struct Ledger {
    pub unspent_outputs: HashMap<UtxoPointer, Output<Address>>,
}
impl Ledger {
    pub fn new(input: HashMap<UtxoPointer, Output<Address>>) -> Self {
        Ledger {
            unspent_outputs: input,
        }
    }
}

/*
#[cfg(test)]
pub mod test {

    use super::*;
    use crate::key::SpendingSecretKey;
    // use cardano::redeem as crypto;
    use chain_addr::{Address, Discrimination, Kind};
    use quickcheck::{Arbitrary, Gen};
    use rand::{CryptoRng, RngCore};

    impl Arbitrary for Ledger {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Ledger {
                unspent_outputs: Arbitrary::arbitrary(g),
            }
        }
    }

    pub fn make_key<R: RngCore + CryptoRng>(rng: &mut R) -> (SpendingSecretKey, Address) {
        let sk = SpendingSecretKey::generate(rng);
        let pk = sk.to_public();
        let user_address = Address(Discrimination::Production, Kind::Single(pk));
        (sk, user_address)
    }

    #[test]
    pub fn tx_no_witness() -> () {
        use chain_core::property::Ledger;
        let mut rng = rand::thread_rng();
        let (_pk1, user1_address) = make_key(&mut rng);
        let tx0_id = TransactionId::hash_bytes(&[0]);
        let value = Value(42000);
        let utxo0 = UtxoPointer {
            transaction_id: tx0_id,
            output_index: 0,
            value: value,
        };
        let ledger = crate::ledger::Ledger::new(
            vec![(utxo0, Output(user1_address.clone(), Value(1)))]
                .iter()
                .cloned()
                .collect(),
        );
        let tx = Transaction {
            inputs: vec![utxo0],
            outputs: vec![Output(user1_address, Value(1))],
        };
        let signed_tx = SignedTransaction {
            transaction: tx,
            witnesses: vec![],
        };
        assert_eq!(
            Err(Error::NotEnoughSignatures(1, 0)),
            ledger.diff_transaction(&signed_tx)
        )
    }

    #[test]
    pub fn tx_wrong_witness() -> () {
        use chain_core::property::Ledger;
        use chain_core::property::Transaction;
        let mut rng = rand::thread_rng();
        let (_, user0_address) = make_key(&mut rng);
        let tx0_id = TransactionId::hash_bytes(&[0]);
        let value = Value(42000);
        let utxo0 = UtxoPointer {
            transaction_id: tx0_id,
            output_index: 0,
            value: value,
        };
        let ledger = crate::ledger::Ledger::new(
            vec![(utxo0, Output(user0_address.clone(), value))]
                .iter()
                .cloned()
                .collect(),
        );
        let output0 = Output(user0_address, value);
        let tx = crate::transaction::Transaction {
            inputs: vec![utxo0],
            outputs: vec![output0.clone()],
        };
        let (pk1, _) = make_key(&mut rng);
        let witness = Witness::new(&tx.id(), &pk1);
        let signed_tx = SignedTransaction {
            transaction: tx,
            witnesses: vec![witness.clone()],
        };
        assert_eq!(
            Err(Error::InvalidSignature(utxo0, output0, witness)),
            ledger.diff_transaction(&signed_tx)
        )
    }

    #[test]
    fn cant_lose_money() {
        use chain_core::property::Ledger;
        use chain_core::property::Transaction;
        let mut rng = rand::thread_rng();
        let (pk1, user1_address) = make_key(&mut rng);
        let tx0_id = TransactionId::hash_bytes(&[0]);
        let value = Value(42000);
        let utxo0 = UtxoPointer {
            transaction_id: tx0_id,
            output_index: 0,
            value: value,
        };
        let ledger = crate::ledger::Ledger::new(
            vec![(utxo0, Output(user1_address.clone(), Value(10)))]
                .iter()
                .cloned()
                .collect(),
        );
        let output0 = Output(user1_address, Value(9));
        let tx = crate::transaction::Transaction {
            inputs: vec![utxo0],
            outputs: vec![output0],
        };
        let witness = Witness::new(&tx.id(), &pk1);
        let signed_tx = SignedTransaction {
            transaction: tx,
            witnesses: vec![witness],
        };
        assert_eq!(
            Err(Error::TransactionSumIsNonZero(10, 9)),
            ledger.diff_transaction(&signed_tx)
        )
    }
}
*/

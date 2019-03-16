//! Mockchain ledger. Ledger exists in order to update the
//! current state and verify transactions.

use crate::transaction::*;
use crate::value::*;
use crate::{account, utxo};
use cardano::address::Addr as OldAddress;
use chain_addr::{Address, Kind};

/// Overall ledger structure.
///
/// This represent a given state related to utxo/old utxo/accounts/... at a given
/// point in time.
///
/// The ledger can be easily and cheaply cloned despite containing refering
/// to a lot of data (millions of utxos, thousands of accounts, ..)
#[derive(Clone)]
pub struct Ledger {
    pub(crate) utxos: utxo::Ledger<Address>,
    pub(crate) oldutxos: utxo::Ledger<OldAddress>,
    pub(crate) accounts: account::Ledger,
}

#[derive(Debug, Clone)]
pub enum Error {
    NotEnoughSignatures(usize, usize),
    UtxoValueNotMatching(Value, Value),
    UtxoError(utxo::Error),
    UtxoInvalidSignature(UtxoPointer, Output<Address>, Witness),
    AccountInvalidSignature(account::Identifier, Witness),
    UtxoInputsTotal(ValueError),
    UtxoOutputsTotal(ValueError),
    Account(account::LedgerError),
    NotBalanced(Value, Value),
    ZeroOutput(Output<Address>),
    ExpectingAccountWitness,
    ExpectingUtxoWitness,
}

impl From<utxo::Error> for Error {
    fn from(e: utxo::Error) -> Self {
        Error::UtxoError(e)
    }
}

impl From<account::LedgerError> for Error {
    fn from(e: account::LedgerError) -> Self {
        Error::Account(e)
    }
}

impl Ledger {
    pub fn new() -> Self {
        Ledger {
            utxos: utxo::Ledger::new(),
            oldutxos: utxo::Ledger::new(),
            accounts: account::Ledger::new(),
        }
    }

    pub fn apply_transaction(
        &mut self,
        signed_tx: &SignedTransaction<Address>,
    ) -> Result<Self, Error> {
        let mut ledger = self.clone();
        let transaction_id = signed_tx.transaction.hash();
        ledger = internal_apply_transaction(
            ledger,
            &transaction_id,
            &signed_tx.transaction.inputs[..],
            &signed_tx.transaction.outputs[..],
            &signed_tx.witnesses[..],
        )?;
        Ok(ledger)
    }
}

/// Apply the transaction
fn internal_apply_transaction(
    mut ledger: Ledger,
    transaction_id: &TransactionId,
    inputs: &[Input],
    outputs: &[Output<Address>],
    witnesses: &[Witness],
) -> Result<Ledger, Error> {
    assert!(inputs.len() < 255);
    assert!(outputs.len() < 255);
    assert!(witnesses.len() < 255);

    // 1. verify that number of signatures matches number of
    // transactions
    if inputs.len() != witnesses.len() {
        return Err(Error::NotEnoughSignatures(inputs.len(), witnesses.len()));
    }

    // 2. validate inputs of transaction by gathering what we know of it,
    // then verifying the associated witness
    for (input, witness) in inputs.iter().zip(witnesses.iter()) {
        match input.to_enum() {
            InputEnum::UtxoInput(utxo) => {
                ledger = input_utxo_verify(ledger, transaction_id, &utxo, witness)?
            }
            InputEnum::AccountInput(account_id, value) => {
                ledger.accounts = input_account_verify(
                    ledger.accounts,
                    transaction_id,
                    &account_id,
                    value,
                    witness,
                )?
            }
        }
    }

    // 3. verify that transaction sum is zero.
    // TODO: with fees this will change
    let total_input =
        Value::sum(inputs.iter().map(|i| i.value)).map_err(|e| Error::UtxoInputsTotal(e))?;
    let total_output =
        Value::sum(inputs.iter().map(|i| i.value)).map_err(|e| Error::UtxoOutputsTotal(e))?;
    if total_input != total_output {
        return Err(Error::NotBalanced(total_input, total_output));
    }

    // 4. add the new outputs
    let mut new_utxos = Vec::new();
    for (index, output) in outputs.iter().enumerate() {
        // Reject zero-valued outputs.
        if output.value == Value::zero() {
            return Err(Error::ZeroOutput(output.clone()));
        }
        match output.address.kind() {
            Kind::Single(_) | Kind::Group(_, _) => {
                new_utxos.push((index as u8, output.clone()));
            }
            Kind::Account(identifier) => {
                // don't have a way to make a newtype ref from the ref so .clone()
                let account = identifier.clone().into();
                ledger.accounts = ledger.accounts.add_value(&account, output.value)?;
            }
        }
    }

    ledger.utxos = ledger.utxos.add(transaction_id, &new_utxos)?;

    Ok(ledger)
}

fn input_utxo_verify(
    mut ledger: Ledger,
    transaction_id: &TransactionId,
    utxo: &UtxoPointer,
    witness: &Witness,
) -> Result<Ledger, Error> {
    match witness {
        Witness::Account(_) => return Err(Error::ExpectingUtxoWitness),
        Witness::Utxo(signature) => {
            let (new_utxos, associated_output) = ledger
                .utxos
                .remove(&utxo.transaction_id, utxo.output_index)?;
            ledger.utxos = new_utxos;
            if utxo.value != associated_output.value {
                return Err(Error::UtxoValueNotMatching(
                    utxo.value,
                    associated_output.value,
                ));
            }

            let verified = signature.verify(
                &associated_output.address.public_key().unwrap(),
                &transaction_id,
            );
            if verified == chain_crypto::Verification::Failed {
                return Err(Error::UtxoInvalidSignature(
                    utxo.clone(),
                    associated_output.clone(),
                    witness.clone(),
                ));
            };
            Ok(ledger)
        }
    }
}

fn input_account_verify(
    mut ledger: account::Ledger,
    transaction_id: &TransactionId,
    account: &account::Identifier,
    value: Value,
    witness: &Witness,
) -> Result<account::Ledger, Error> {
    // .remove_value() check if there's enough value and if not, returns a Err.
    let (new_ledger, spending_counter) = ledger.remove_value(account, value)?;
    ledger = new_ledger;

    match witness {
        Witness::Utxo(_) => return Err(Error::ExpectingAccountWitness),
        Witness::Account(sig) => {
            let tidsc = TransactionIdSpendingCounter::new(transaction_id, &spending_counter);
            let verified = sig.verify(&account.clone().into(), &tidsc);
            if verified == chain_crypto::Verification::Failed {
                return Err(Error::AccountInvalidSignature(
                    account.clone(),
                    witness.clone(),
                ));
            };
            Ok(ledger)
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

//! Mockchain ledger. Ledger exists in order to update the
//! current state and verify transactions.

use crate::fee::LinearFee;
use crate::legacy;
use crate::transaction::*;
use crate::value::*;
use crate::{account, utxo};
use chain_addr::{Address, Discrimination, Kind};
use chain_core::property;

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
    pub(crate) oldutxos: utxo::Ledger<legacy::OldAddress>,
    pub(crate) accounts: account::Ledger,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    NotEnoughSignatures(usize, usize),
    UtxoValueNotMatching(Value, Value),
    UtxoError(utxo::Error),
    UtxoInvalidSignature(UtxoPointer, Output<Address>, Witness),
    OldUtxoInvalidSignature(UtxoPointer, Output<legacy::OldAddress>, Witness),
    OldUtxoInvalidPublicKey(UtxoPointer, Output<legacy::OldAddress>, Witness),
    AccountInvalidSignature(account::Identifier, Witness),
    UtxoInputsTotal(ValueError),
    UtxoOutputsTotal(ValueError),
    Account(account::LedgerError),
    NotBalanced(Value, Value),
    ZeroOutput(Output<Address>),
    InvalidDiscrimination,
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
        &self,
        signed_tx: &SignedTransaction<Address>,
        allow_account_creation: bool,
        linear_fees: &LinearFee,
        discrimination: &Discrimination,
    ) -> Result<Self, Error> {
        let mut ledger = self.clone();
        let transaction_id = signed_tx.transaction.hash();
        ledger = internal_apply_transaction(
            ledger,
            allow_account_creation,
            linear_fees,
            discrimination,
            &transaction_id,
            &signed_tx.transaction.inputs[..],
            &signed_tx.transaction.outputs[..],
            &signed_tx.witnesses[..],
        )?;
        Ok(ledger)
    }
}

impl property::Ledger<SignedTransaction<Address>> for Ledger {
    type Error = Error;

    fn input<'a, I>(&'a self, input: Input) -> Result<&'a Output<Address>, Self::Error> {
        match input.to_enum() {
            InputEnum::AccountInput(_, _) => {
                Err(Error::UtxoError(utxo::Error::TransactionNotFound))
            }
            InputEnum::UtxoInput(utxo_ptr) => self
                .utxos
                .get(&utxo_ptr.transaction_id, &utxo_ptr.output_index)
                .map(|entry| Ok(entry.output))
                .unwrap_or_else(|| Err(Error::UtxoError(utxo::Error::TransactionNotFound))),
        }
    }
}

/// Apply the transaction
fn internal_apply_transaction(
    mut ledger: Ledger,
    allow_account_creation: bool,
    linear_fees: &LinearFee,
    discrimination: &Discrimination,
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

        if output.address.discrimination() != *discrimination {
            return Err(Error::InvalidDiscrimination);
        }
        match output.address.kind() {
            Kind::Single(_) | Kind::Group(_, _) => {
                new_utxos.push((index as u8, output.clone()));
            }
            Kind::Account(identifier) => {
                // don't have a way to make a newtype ref from the ref so .clone()
                let account = identifier.clone().into();
                ledger.accounts = match ledger.accounts.add_value(&account, output.value) {
                    Ok(accounts) => accounts,
                    Err(account::LedgerError::NonExistent) if allow_account_creation => {
                        // if the account was not existent and that we allow creating
                        // account out of the blue, then fallback on adding the account
                        ledger.accounts.add_account(&account, output.value)?
                    }
                    Err(error) => return Err(error.into()),
                };
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
        Witness::OldUtxo(xpub, signature) => {
            let (old_utxos, associated_output) = ledger
                .oldutxos
                .remove(&utxo.transaction_id, utxo.output_index)?;

            ledger.oldutxos = old_utxos;
            if utxo.value != associated_output.value {
                return Err(Error::UtxoValueNotMatching(
                    utxo.value,
                    associated_output.value,
                ));
            };

            if legacy::oldaddress_from_xpub(&associated_output.address, xpub) {
                return Err(Error::OldUtxoInvalidPublicKey(
                    utxo.clone(),
                    associated_output.clone(),
                    witness.clone(),
                ));
            };

            let verified = signature.verify(&xpub, &transaction_id);
            if verified == chain_crypto::Verification::Failed {
                return Err(Error::OldUtxoInvalidSignature(
                    utxo.clone(),
                    associated_output.clone(),
                    witness.clone(),
                ));
            };

            Ok(ledger)
        }
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
        Witness::OldUtxo(_, _) => return Err(Error::ExpectingAccountWitness),
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

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for Error {}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::key::{SpendingPublicKey, SpendingSecretKey};
    use chain_addr::{Address, Discrimination, Kind};
    use rand::{CryptoRng, RngCore};

    pub fn make_key<R: RngCore + CryptoRng>(
        rng: &mut R,
        discrimination: &Discrimination,
    ) -> (SpendingSecretKey, SpendingPublicKey, Address) {
        let sk = SpendingSecretKey::generate(rng);
        let pk = sk.to_public();
        let user_address = Address(discrimination.clone(), Kind::Single(pk.clone()));
        (sk, pk, user_address)
    }

    macro_rules! assert_err {
        ($left: expr, $right: expr) => {
            match &($left) {
                left_val => match &($right) {
                    Err(e) => {
                        if !(e == left_val) {
                            panic!(
                                "assertion failed: error mismatch \
                                 (left: `{:?}, right: `{:?}`)",
                                *left_val, *e
                            )
                        }
                    }
                    Ok(_) => panic!(
                        "assertion failed: expected error {:?} but got success",
                        *left_val
                    ),
                },
            }
        };
    }

    #[test]
    pub fn utxo() -> () {
        let no_fee = LinearFee::new(0, 0, 0);
        let discrimination = Discrimination::Test;
        let mut rng = rand::thread_rng();
        let (sk1, _pk1, user1_address) = make_key(&mut rng, &discrimination);
        let (_sk2, _pk2, user2_address) = make_key(&mut rng, &discrimination);
        let tx0_id = TransactionId::hash_bytes(&[0]);
        let value = Value(42000);

        let output0 = Output {
            address: user1_address.clone(),
            value: value,
        };

        let utxo0 = UtxoPointer {
            transaction_id: tx0_id,
            output_index: 0,
            value: value,
        };
        let ledger = {
            let mut l = Ledger::new();
            l.utxos = l.utxos.add(&tx0_id, &[(0, output0)]).unwrap();
            l
        };

        {
            let tx = Transaction {
                inputs: vec![Input::from_utxo(utxo0)],
                outputs: vec![Output {
                    address: user2_address.clone(),
                    value: Value(1),
                }],
            };
            let signed_tx = SignedTransaction {
                transaction: tx,
                witnesses: vec![],
            };
            let r = ledger.apply_transaction(&signed_tx, false, &no_fee, &discrimination);
            assert_err!(Error::NotEnoughSignatures(1, 0), r)
        }

        {
            let tx = Transaction {
                inputs: vec![Input::from_utxo(utxo0)],
                outputs: vec![Output {
                    address: user2_address.clone(),
                    value: Value(1),
                }],
            };
            let txid = tx.hash();
            let w1 = Witness::new(&txid, &sk1);
            let signed_tx = SignedTransaction {
                transaction: tx,
                witnesses: vec![w1],
            };
            let r = ledger.apply_transaction(&signed_tx, false, &no_fee, &discrimination);
            assert!(r.is_ok())
        }
    }
}

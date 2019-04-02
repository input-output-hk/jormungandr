use crate::certificate as cert;
use crate::fee::FeeAlgorithm;
use crate::transaction as tx;
use crate::value::{Value, ValueError};
use chain_addr::Address;
use std::{error, fmt};

/// Possible error for the builder.
#[derive(Debug, Clone)]
pub enum Error {
    TxInvalidNoInput,
    TxInvalidNoOutput,
    TxNotEnoughTotalInput,
    MathErr(ValueError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::TxInvalidNoInput => write!(f, "transaction has no inputs"),
            Error::TxInvalidNoOutput => write!(f, "transaction has no outputs"),
            Error::TxNotEnoughTotalInput => write!(f, "not enough input for making transaction"),
            Error::MathErr(v) => write!(f, "error in arithmetics {:?}", v),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

/// Output policy to be used in transaction. This policy is used then
/// there is positive balance on in the OUTPUT+FEE-INPUT. Policy
/// explains how to use that balance. Rember that policy application
/// may change the amount of the fee.
#[derive(Debug, Clone)]
pub enum OutputPolicy {
    /// Send all extra balance to the given address.
    One(Address),
    /// Forget everything, do not try to return money.
    Forget,
}

#[derive(Clone)]
/// Transaction builder is an object to construct
/// a transaction with iterative steps (inputs, outputs)
pub struct TransactionBuilder<Address, Extra> {
    tx: tx::Transaction<Address, Extra>,
}

impl TransactionBuilder<Address, tx::NoExtra> {
    pub fn new() -> Self {
        TransactionBuilder {
            tx: tx::Transaction {
                inputs: vec![],
                outputs: vec![],
                extra: tx::NoExtra,
            },
        }
    }

    pub fn set_certificate(
        self,
        certificate: cert::Certificate,
    ) -> TransactionBuilder<Address, cert::Certificate> {
        TransactionBuilder {
            tx: self.tx.replace_extra(certificate),
        }
    }
}

impl<Extra: Clone> TransactionBuilder<Address, Extra> {
    /// Create new transaction builder.

    /// Add additional input.
    ///
    /// Each input may extend the size of the required fee.
    pub fn add_input(&mut self, input: &tx::Input) {
        self.tx.inputs.push(input.clone())
    }

    /// Add additional output.
    ///
    /// Each output may extend the size of the required fee.
    pub fn add_output(&mut self, address: Address, value: Value) {
        self.tx.outputs.push(tx::Output { address, value })
    }

    pub fn estimate_fee<F: FeeAlgorithm>(&self, fee_algorithm: F) -> Result<Value, ValueError> {
        fee_algorithm
            .calculate_for(&self.tx)
            .ok_or(ValueError::Overflow)
    }

    /// Get balance including current feee.
    pub fn get_balance<F: FeeAlgorithm>(&self, fee_algorithm: F) -> Result<Balance, ValueError> {
        let fee = fee_algorithm
            .calculate_for(&self.tx)
            .ok_or(ValueError::Overflow)?;
        balance(&self.tx, fee)
    }

    /// Get transaction balance without fee included.
    pub fn get_balance_without_fee(&self) -> Result<Balance, ValueError> {
        balance(&self.tx, Value(0))
    }

    /// Create transaction finalizer without performing any
    /// checks or output balancing.
    pub fn unchecked_finalize(self) -> tx::Transaction<Address, Extra> {
        self.tx
    }

    /// We finalize the transaction by passing fee rule and return
    /// policy. Then after all calculations were made we can get
    /// the information back to us.
    ///
    pub fn finalize<F: FeeAlgorithm>(
        mut self,
        fee_algorithm: F,
        policy: OutputPolicy,
    ) -> Result<(Balance, tx::Transaction<Address, Extra>), Error> {
        if self.tx.inputs.len() == 0 {
            return Err(Error::TxInvalidNoInput);
        }
        if self.tx.outputs.len() == 0 {
            return Err(Error::TxInvalidNoOutput);
        }
        // calculate initial fee, maybe we can fit it without any
        // additional calculations.
        let fee = fee_algorithm
            .calculate_for(&self.tx)
            .ok_or(Error::MathErr(ValueError::Overflow))?;
        let pos = match balance(&self.tx, fee) {
            Ok(Balance::Negative(_)) => return Err(Error::TxNotEnoughTotalInput),
            Ok(Balance::Positive(v)) => v,
            Ok(Balance::Zero) => {
                return Ok((Balance::Zero, self.tx));
            }
            Err(err) => return Err(Error::MathErr(err)),
        };
        // we have more money in the inputs then fee and outputs
        // so we need to return some money back to us.
        match policy {
            OutputPolicy::Forget => Ok((Balance::Positive(pos), self.tx)),
            // We will try to find the best matching value, for
            // this reason we will try to reduce the set using
            // value estimated by the current fee.
            //
            // We are searching in a range
            //   [min_value, max_value)
            OutputPolicy::One(address) => {
                // This is simplified version of the algorithm that
                // works only in case in fee can perfectly estimate
                // the required cost. We add an additional empty output
                // hoping that it doesn't change fee count.
                //
                // Otherwise better estimation algorithm is needed.
                let mut tx = self.tx.clone();
                tx.outputs.push(tx::Output {
                    address: address.clone(),
                    value: Value(0),
                });
                let fee = fee_algorithm
                    .calculate_for(&tx)
                    .ok_or(Error::MathErr(ValueError::Overflow))?;
                match balance(&tx, fee) {
                    Ok(Balance::Positive(value)) => {
                        self.tx.outputs.push(tx::Output { address, value });
                        Ok((Balance::Zero, self.tx))
                    }
                    _ => Ok((Balance::Positive(pos), self.tx)),
                }
            }
        }
    }
}

/// Amount of the balance in the transaction.
pub enum Balance {
    /// Balance is positive.
    Positive(Value),
    /// Balance is negative, such transaction can't be valid.
    Negative(Value),
    /// Balance is zero.
    Zero,
}

fn balance<Extra>(tx: &tx::Transaction<Address, Extra>, fee: Value) -> Result<Balance, ValueError> {
    let inputs = Value::sum(tx.inputs.iter().map(|i| i.value))?;
    let outputs = Value::sum(tx.outputs.iter().map(|o| o.value))?;
    let z = (outputs + fee)?;
    if inputs > z {
        Ok(Balance::Positive((inputs - z)?))
    } else if inputs < z {
        Ok(Balance::Negative((z - inputs)?))
    } else {
        Ok(Balance::Zero)
    }
}

pub enum TransactionFinalizer {
    Type1(
        tx::Transaction<Address, tx::NoExtra>,
        Vec<Option<tx::Witness>>,
    ),
    Type2(
        tx::Transaction<Address, cert::Certificate>,
        Vec<Option<tx::Witness>>,
    ),
}

pub enum GeneratedTransaction {
    Type1(tx::AuthenticatedTransaction<Address, tx::NoExtra>),
    Type2(tx::AuthenticatedTransaction<Address, cert::Certificate>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    WitnessOutOfBound(usize, usize),
    WitnessMismatch(usize),
    MissingWitnessAt(usize),
}

fn set_witness<Address, Extra>(
    transaction: &tx::Transaction<Address, Extra>,
    witnesses: &mut Vec<Option<tx::Witness>>,
    index: usize,
    witness: tx::Witness,
) -> Result<(), BuildError> {
    if index >= witnesses.len() {
        return Err(BuildError::WitnessOutOfBound(index, witnesses.len()));
    }

    match (transaction.inputs[index].get_type(), &witness) {
        (tx::InputType::Utxo, tx::Witness::OldUtxo(_, _)) => (),
        (tx::InputType::Utxo, tx::Witness::Utxo(_)) => (),
        (tx::InputType::Account, tx::Witness::Account(_)) => (),
        (_, _) => return Err(BuildError::WitnessMismatch(index)),
    };

    witnesses[index] = Some(witness);
    Ok(())
}

fn get_full_witnesses(witnesses: Vec<Option<tx::Witness>>) -> Result<Vec<tx::Witness>, BuildError> {
    let mut v = Vec::new();
    for (i, w) in witnesses.iter().enumerate() {
        match w {
            None => return Err(BuildError::MissingWitnessAt(i)),
            Some(w) => v.push(w.clone()),
        }
    }
    Ok(v)
}

impl TransactionFinalizer {
    pub fn new_trans(transaction: tx::Transaction<Address, tx::NoExtra>) -> Self {
        let nb_inputs = transaction.inputs.len();
        TransactionFinalizer::Type1(transaction, vec![None; nb_inputs])
    }

    pub fn new_cert(transaction: tx::Transaction<Address, cert::Certificate>) -> Self {
        let nb_inputs = transaction.inputs.len();
        TransactionFinalizer::Type2(transaction, vec![None; nb_inputs])
    }

    pub fn set_witness(&mut self, index: usize, witness: tx::Witness) -> Result<(), BuildError> {
        match self {
            TransactionFinalizer::Type1(ref t, ref mut w) => set_witness(t, w, index, witness),
            TransactionFinalizer::Type2(ref t, ref mut w) => set_witness(t, w, index, witness),
        }
    }

    pub fn get_txid(&self) -> tx::TransactionId {
        match self {
            TransactionFinalizer::Type1(t, _) => t.hash(),
            TransactionFinalizer::Type2(t, _) => t.hash(),
        }
    }

    pub fn build(self) -> Result<GeneratedTransaction, BuildError> {
        match self {
            TransactionFinalizer::Type1(t, witnesses) => {
                Ok(GeneratedTransaction::Type1(tx::AuthenticatedTransaction {
                    transaction: t,
                    witnesses: get_full_witnesses(witnesses)?,
                }))
            }
            TransactionFinalizer::Type2(t, witnesses) => {
                Ok(GeneratedTransaction::Type2(tx::AuthenticatedTransaction {
                    transaction: t,
                    witnesses: get_full_witnesses(witnesses)?,
                }))
            }
        }
    }
}

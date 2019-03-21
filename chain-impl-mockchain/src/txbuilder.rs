use crate::certificate as cert;
use crate::fee::FeeAlgorithm;
use crate::key::SpendingSecretKey;
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
/// Transacion builder is helper to generate well
/// formed transaction in for the blockchain.
pub struct TransactionBuilder<Address> {
    tx: tx::Transaction<Address>,
    cert: Option<cert::Certificate>,
}

impl TransactionBuilder<Address> {
    /// Create new transaction builder.
    pub fn new() -> TransactionBuilder<Address> {
        TransactionBuilder {
            tx: tx::Transaction {
                inputs: vec![],
                outputs: vec![],
            },
            cert: None,
        }
    }

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

    pub fn set_certificate(&mut self, certificate: cert::Certificate) {
        self.cert = Some(certificate);
    }

    pub fn estimate_fee<F: FeeAlgorithm>(&self, fee_algorithm: F) -> Result<Value, ValueError> {
        fee_algorithm
            .calculate_for(&self.tx, &self.cert)
            .ok_or(ValueError::Overflow)
    }

    /// Get balance including current feee.
    pub fn get_balance<F: FeeAlgorithm>(&self, fee_algorithm: F) -> Result<Balance, ValueError> {
        let fee = fee_algorithm
            .calculate_for(&self.tx, &self.cert)
            .ok_or(ValueError::Overflow)?;
        balance(&self.tx, fee)
    }

    /// Get transaction balance without fee included.
    pub fn get_balance_without_fee(&self) -> Result<Balance, ValueError> {
        balance(&self.tx, Value(0))
    }

    /// Create transaction finalizer without performing any
    /// checks or output balancing.
    pub fn unchecked_finalize(mut self) -> TransactionFinalizer {
        TransactionFinalizer::new(self.tx, self.cert)
    }

    /// We finalize the transaction by passing fee rule and return
    /// policy. Then after all calculations were made we can get
    /// the information back to us.
    ///
    pub fn finalize<F: FeeAlgorithm>(
        mut self,
        fee_algorithm: F,
        policy: OutputPolicy,
    ) -> Result<(Balance, TransactionFinalizer), Error> {
        if self.tx.inputs.len() == 0 {
            return Err(Error::TxInvalidNoInput);
        }
        if self.tx.outputs.len() == 0 {
            return Err(Error::TxInvalidNoOutput);
        }
        // calculate initial fee, maybe we can fit it without any
        // additional calculations.
        let fee = fee_algorithm
            .calculate_for(&self.tx, &self.cert)
            .ok_or(Error::MathErr(ValueError::Overflow))?;
        let pos = match balance(&self.tx, fee) {
            Ok(Balance::Negative(_)) => return Err(Error::TxNotEnoughTotalInput),
            Ok(Balance::Positive(v)) => v,
            Ok(Balance::Zero) => {
                return Ok((Balance::Zero, TransactionFinalizer::new(self.tx, self.cert)));
            }
            Err(err) => return Err(Error::MathErr(err)),
        };
        // we have more money in the inputs then fee and outputs
        // so we need to return some money back to us.
        match policy {
            OutputPolicy::Forget => {
                let tx = TransactionFinalizer::new(self.tx, self.cert);
                Ok((Balance::Positive(pos), tx))
            }
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
                    .calculate_for(&tx, &self.cert)
                    .ok_or(Error::MathErr(ValueError::Overflow))?;
                match balance(&tx, fee) {
                    Ok(Balance::Positive(value)) => {
                        self.tx.outputs.push(tx::Output { address, value });
                        Ok((Balance::Zero, TransactionFinalizer::new(self.tx, self.cert)))
                    }
                    _ => Ok((
                        Balance::Positive(pos),
                        TransactionFinalizer::new(self.tx, self.cert),
                    )),
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

fn balance(tx: &tx::Transaction<Address>, fee: Value) -> Result<Balance, ValueError> {
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
    Type1(tx::SignedTransaction<Address>),
    Type2(tx::SignedCertificateTransaction<Address>),
}

pub enum GeneratedTransaction {
    Type1(tx::SignedTransaction<Address>),
    Type2(tx::SignedCertificateTransaction<Address>),
}

impl TransactionFinalizer {
    fn new(transaction: tx::Transaction<Address>, certificate: Option<cert::Certificate>) -> Self {
        match certificate {
            Some(certificate) => TransactionFinalizer::Type2(tx::SignedCertificateTransaction {
                transaction: tx::CertificateTransaction {
                    transaction,
                    certificate,
                },
                witnesses: vec![],
            }),
            None => TransactionFinalizer::Type1(tx::SignedTransaction {
                transaction,
                witnesses: vec![],
            }),
        }
    }

    /// Sign transaction.
    pub fn sign(&mut self, pk: &SpendingSecretKey) {
        // TODO: check if signature is required.
        // TODO: check if signature matches address.
        let id = match &self {
            TransactionFinalizer::Type1(t) => t.transaction.hash(),
            TransactionFinalizer::Type2(t) => t.transaction.hash(),
        };
        let witness = tx::Witness::new(&id, pk);
        match self {
            TransactionFinalizer::Type1(t) => t.witnesses.push(witness),
            TransactionFinalizer::Type2(t) => t.witnesses.push(witness),
        }
    }

    pub fn build(self) -> GeneratedTransaction {
        match self {
            TransactionFinalizer::Type1(t) => GeneratedTransaction::Type1(t),
            TransactionFinalizer::Type2(t) => GeneratedTransaction::Type2(t),
        }
    }
}

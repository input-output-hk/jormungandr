use super::{Balance, Input, Output, Payload};
use crate::fee::FeeAlgorithm;
use crate::value::{Value, ValueError};
use chain_addr::Address;
use std::error;
use std::fmt;

/// Inputs & Outputs for a transaction being built
pub struct InputOutputBuilder {
    inputs: Vec<Input>,
    outputs: Vec<Output<Address>>,
}

/// Inputs & Outputs for a built transaction
pub struct InputOutput {
    pub inputs: Box<[Input]>,
    pub outputs: Box<[Output<Address>]>,
}

/// Possible error for the builder.
#[derive(Debug, Clone)]
pub enum Error {
    TxInvalidNoInput,
    TxInvalidNoOutput,
    TxNotEnoughTotalInput,
    TxTooMuchTotalInput,
    MathErr(ValueError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::TxInvalidNoInput => write!(f, "transaction has no inputs"),
            Error::TxInvalidNoOutput => write!(f, "transaction has no outputs"),
            Error::TxNotEnoughTotalInput => write!(f, "not enough input for making transaction"),
            Error::TxTooMuchTotalInput => write!(f, "too muny input value for making transaction"),
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

impl InputOutputBuilder {
    /// Create a new empty builder
    pub fn empty() -> InputOutputBuilder {
        InputOutputBuilder {
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    /// Create a builder from a given sequence of inputs and outputs
    pub fn new<'a, IITER, OITER>(inputs: IITER, outputs: OITER) -> InputOutputBuilder
    where
        IITER: Iterator<Item = &'a Input>,
        OITER: Iterator<Item = &'a Output<Address>>,
    {
        let inputs = inputs.cloned().collect();
        let outputs = outputs.cloned().collect();
        InputOutputBuilder { inputs, outputs }
    }

    /// Build the InputOutput from the Builder
    pub fn build(self) -> InputOutput {
        InputOutput {
            inputs: self.inputs.into(),
            outputs: self.outputs.into(),
        }
    }

    /// Add additional input.
    ///
    /// Each input may extend the size of the required fee.
    pub fn add_input(&mut self, input: &Input) {
        self.inputs.push(input.clone())
    }

    /// Add additional output.
    ///
    /// Each output may extend the size of the required fee.
    pub fn add_output(&mut self, address: Address, value: Value) {
        self.outputs.push(Output { address, value })
    }

    pub fn balance(&self, fee: Value) -> Result<Balance, ValueError> {
        let inputs = Value::sum(self.inputs.iter().map(|i| i.value()))?;
        let outputs = Value::sum(self.outputs.iter().map(|o| o.value))?;
        let z = (outputs + fee)?;
        if inputs > z {
            Ok(Balance::Positive((inputs - z)?))
        } else if inputs < z {
            Ok(Balance::Negative((z - inputs)?))
        } else {
            Ok(Balance::Zero)
        }
    }

    /// Calculate the fees on a given fee algorithm for the current transaction
    pub fn estimate_fee<P: Payload, F: FeeAlgorithm<P>>(
        &self,
        payload: &P,
        fee_algorithm: F,
    ) -> Result<Value, ValueError> {
        fee_algorithm
            .calculate(payload, &self.inputs, &self.outputs)
            .ok_or(ValueError::Overflow)
    }

    /// Get balance including current fee.
    pub fn get_balance<P: Payload, F: FeeAlgorithm<P>>(
        &self,
        payload: &P,
        fee_algorithm: F,
    ) -> Result<Balance, ValueError> {
        let fee = fee_algorithm
            .calculate(payload, &self.inputs, &self.outputs)
            .ok_or(ValueError::Overflow)?;
        self.balance(fee)
    }

    /// Get transaction balance without fee included.
    pub fn get_balance_without_fee(&self) -> Result<Balance, ValueError> {
        self.balance(Value::zero())
    }

    /// Seal the transaction checking that the transaction fits the fee algorithm
    pub fn seal<P: Payload, F: FeeAlgorithm<P>>(
        self,
        payload: &P,
        fee_algorithm: F,
    ) -> Result<InputOutput, Error> {
        match self.get_balance(payload, fee_algorithm) {
            Err(err) => Err(Error::MathErr(err)),
            Ok(Balance::Negative(_)) => Err(Error::TxNotEnoughTotalInput),
            Ok(Balance::Positive(_)) => Err(Error::TxTooMuchTotalInput),
            Ok(Balance::Zero) => Ok(self.build()),
        }
    }

    /// Seal the transaction by passing fee rule and the output policy
    ///
    /// Along with the transaction, this return the balance unassigned to output policy
    /// if any
    pub fn seal_with_output_policy<P: Payload, F: FeeAlgorithm<P>>(
        mut self,
        payload: &P,
        fee_algorithm: F,
        policy: OutputPolicy,
    ) -> Result<(Balance, Vec<Output<Address>>, InputOutput), Error> {
        // calculate initial fee, maybe we can fit it without any
        // additional calculations.
        let fee = fee_algorithm
            .calculate(payload, &self.inputs, &self.outputs)
            .ok_or(Error::MathErr(ValueError::Overflow))?;
        let pos = match self.balance(fee) {
            Ok(Balance::Negative(_)) => return Err(Error::TxNotEnoughTotalInput),
            Ok(Balance::Positive(v)) => v,
            Ok(Balance::Zero) => {
                return Ok((Balance::Zero, vec![], self.build()));
            }
            Err(err) => return Err(Error::MathErr(err)),
        };
        // we have more money in the inputs then fee and outputs
        // so we need to return some money back to us.
        match policy {
            OutputPolicy::Forget => Ok((Balance::Positive(pos), vec![], self.build())),
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
                self.outputs.push(Output {
                    address: address.clone(),
                    value: Value(0),
                });
                let fee = fee_algorithm
                    .calculate(payload, &self.inputs, &self.outputs)
                    .ok_or(Error::MathErr(ValueError::Overflow))?;
                match self.balance(fee) {
                    Ok(Balance::Positive(value)) => {
                        let _ = self.outputs.pop();
                        let output = Output { address, value };
                        self.outputs.push(output.clone());
                        Ok((Balance::Zero, vec![output], self.build()))
                    }
                    _ => {
                        let _ = self.outputs.pop();
                        Ok((Balance::Positive(pos), vec![], self.build()))
                    }
                }
            }
        }
    }
}

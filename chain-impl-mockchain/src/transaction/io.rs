use super::{Balance, Input, Output, Payload, PayloadSlice};
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
    TxTooManyInputs,
    TxTooManyOutputs,
    TxNotEnoughTotalInput,
    TxTooMuchTotalInput,
    MathErr(ValueError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::TxInvalidNoInput => write!(f, "transaction has no inputs"),
            Error::TxInvalidNoOutput => write!(f, "transaction has no outputs"),
            Error::TxTooManyInputs => write!(f, "transaction has too many inputs"),
            Error::TxTooManyOutputs => write!(f, "transaction has too many outputs"),
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
    pub fn add_input(&mut self, input: &Input) -> Result<(), Error> {
        if self.inputs.len() == 256 {
            return Err(Error::TxTooManyInputs);
        }
        self.inputs.push(input.clone());
        Ok(())
    }

    /// Add additional output.
    ///
    /// Each output may extend the size of the required fee.
    pub fn add_output(&mut self, address: Address, value: Value) -> Result<(), Error> {
        if self.outputs.len() == 256 {
            return Err(Error::TxTooManyOutputs);
        }
        self.outputs.push(Output { address, value });
        Ok(())
    }

    /// Remove input at the index specified starting from the oldest added input.
    pub fn remove_input(&mut self, input: usize) {
        if input < self.inputs.len() {
            let _ = self.inputs.remove(input);
        }
    }

    /// Remove output at the index specified starting from the oldest added output.
    pub fn remove_output(&mut self, output: usize) {
        if output < self.outputs.len() {
            let _ = self.outputs.remove(output);
        }
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
    pub fn estimate_fee<'a, P: Payload, F: FeeAlgorithm>(
        &self,
        payload: PayloadSlice<'a, P>,
        fee_algorithm: &F,
    ) -> Value {
        fee_algorithm.calculate(
            payload.to_certificate_slice(),
            self.inputs.len() as u8,
            self.outputs.len() as u8,
        )
    }

    /// Get balance including current fee.
    pub fn get_balance<'a, P: Payload, F: FeeAlgorithm>(
        &self,
        payload: PayloadSlice<'a, P>,
        fee_algorithm: &F,
    ) -> Result<Balance, ValueError> {
        let fee = self.estimate_fee(payload, fee_algorithm);
        self.balance(fee)
    }

    /// Get transaction balance without fee included.
    pub fn get_balance_without_fee(&self) -> Result<Balance, ValueError> {
        self.balance(Value::zero())
    }

    /// Seal the transaction checking that the transaction fits the fee algorithm
    pub fn seal<'a, P: Payload, F: FeeAlgorithm>(
        self,
        payload: PayloadSlice<'a, P>,
        fee_algorithm: &F,
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
    pub fn seal_with_output_policy<'a, P: Payload, F: FeeAlgorithm>(
        mut self,
        payload: PayloadSlice<'a, P>,
        fee_algorithm: &F,
        policy: OutputPolicy,
    ) -> Result<(Balance, Vec<Output<Address>>, InputOutput), Error> {
        // calculate initial fee, maybe we can fit it without any
        // additional calculations.
        let fee = self.estimate_fee(payload.clone(), fee_algorithm);
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
                self.add_output(address.clone(), Value::zero())?;
                let fee = self.estimate_fee(payload, fee_algorithm);
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

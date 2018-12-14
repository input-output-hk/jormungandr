use crate::blockcfg::chain::mock::{Transaction, Input, Output, Signature};
use crate::blockcfg::ledger;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Ledger {
    unspent_outputs: HashMap<Input, Output>,
}
impl Ledger {
    pub fn new() -> Self {
        Ledger {
            unspent_outputs: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Diff {
    spent_outputs: HashMap<Input, Output>,
    new_unspent_outputs: HashMap<Input, Output>,
}
impl Diff {
    fn new() -> Self {
        Diff {
            spent_outputs: HashMap::new(),
            new_unspent_outputs: HashMap::new(),
        }
    }

    fn extend(&mut self, other: Self) {
        self.new_unspent_outputs.extend(other.new_unspent_outputs);
        self.spent_outputs.extend(other.spent_outputs);
    }
}

#[derive(Debug, Clone)]
pub enum Error {
    /// If the Ledger could not find the given input in the UTxO list it will
    /// report this error.
    InputDoesNotResolve(Input),

    /// if the Ledger finds that the input has already been used once in a given
    /// transaction or block of transactions it will report this error.
    ///
    /// the input here is the given input used twice,
    /// the output here is the output set in the first occurrence of the input, it
    /// will provide a bit of information to the user to figure out what went wrong
    DoubleSpend(Input, Output),

    /// This error will happen if the input was already set and is now replaced
    /// by another output.
    ///
    /// I.E: the value output has changed but the input is the same. This should not
    /// happen since changing the output will change the transaction identifier
    /// associated to this output.
    ///
    /// first the input in common, then the original output and finally the new output
    InputWasAlreadySet(Input, Output, Output),

    /// error occurs if the signature is invalid: either does not match the initial output
    /// or it is not cryptographically valid.
    InvalidSignature(Input, Output, Signature),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::InputDoesNotResolve(_) => write!(f, "Input does not resolve to an UTxO"),
            Error::DoubleSpend(_, _) => write!(f, "UTxO spent twice in the same transaction"),
            Error::InputWasAlreadySet(_, _, _) => write!(f, "Input was already present in the Ledger"),
            Error::InvalidSignature(_, _, _) => write!(f, "Input is not signed properly"),
        }
    }
}
impl std::error::Error for Error {}

impl ledger::Ledger for Ledger {
    type Transaction = Transaction;
    type Diff = Diff;
    type Error = Error;

    fn diff_transaction(&self, transaction: &Self::Transaction) -> Result<Self::Diff, Self::Error> {
        use crate::blockcfg::ledger::generic::Transaction;

        let mut diff = Diff::new();
        let id = transaction.id();

        // 1. validate the inputs
        for input in transaction.inputs.iter() {
            if let Some(output) = self.unspent_outputs.get(&input.input) {
                if ! input.verify(&output) {
                    return Err(Error::InvalidSignature(input.input, *output, input.signature.clone()));
                }
                if let Some(output) = diff.spent_outputs.insert(input.input, *output) {
                    return Err(Error::DoubleSpend(input.input, output));
                }

            } else {
                return Err(Error::InputDoesNotResolve(input.input));
            }
        }

        // 2. prepare to add the new outputs
        for (index, output) in transaction.outputs.iter().enumerate() {
            diff.new_unspent_outputs.insert(
                Input::new(id, index as u32),
                *output
            );
        }

        Ok(diff)
    }

    fn diff<'a, I>(&self, transactions: I) -> Result<Self::Diff, Self::Error>
        where I: Iterator<Item = &'a Self::Transaction> + Sized
            , Self::Transaction: 'a
    {
        let mut diff = Diff::new();

        for transaction in transactions {
            diff.extend(self.diff_transaction(transaction)?);
        }

        Ok(diff)
    }

    fn add(&mut self, diff: Self::Diff) -> Result<&mut Self, Self::Error> {
        for spent_output in diff.spent_outputs.keys() {
            if let None = self.unspent_outputs.remove(spent_output) {
                return Err(Error::InputDoesNotResolve(*spent_output));
            }
        }

        for (input, output) in diff.new_unspent_outputs {
            if let Some(original_output) = self.unspent_outputs.insert(input, output) {
                return Err(Error::InputWasAlreadySet(input, original_output, output));
            }
        }

        Ok(self)
    }
}

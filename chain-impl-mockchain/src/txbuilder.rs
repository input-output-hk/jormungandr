use crate::certificate as cert;
use crate::fee::FeeAlgorithm;
use crate::transaction::{self as tx, Balance};
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

#[derive(Clone, Debug)]
/// Transaction builder is an object to construct
/// a transaction with iterative steps (inputs, outputs)
pub struct TransactionBuilder<Address, Extra> {
    pub tx: tx::Transaction<Address, Extra>,
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

impl<Address, Extra> From<tx::Transaction<Address, Extra>> for TransactionBuilder<Address, Extra> {
    fn from(tx: tx::Transaction<Address, Extra>) -> Self {
        TransactionBuilder { tx }
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

    pub fn estimate_fee<F: FeeAlgorithm<tx::Transaction<Address, Extra>>>(
        &self,
        fee_algorithm: F,
    ) -> Result<Value, ValueError> {
        fee_algorithm
            .calculate(&self.tx)
            .ok_or(ValueError::Overflow)
    }

    /// Get balance including current feee.
    pub fn get_balance<F: FeeAlgorithm<tx::Transaction<Address, Extra>>>(
        &self,
        fee_algorithm: F,
    ) -> Result<Balance, ValueError> {
        let fee = fee_algorithm
            .calculate(&self.tx)
            .ok_or(ValueError::Overflow)?;
        self.tx.balance(fee)
    }

    /// Get transaction balance without fee included.
    pub fn get_balance_without_fee(&self) -> Result<Balance, ValueError> {
        self.tx.balance(Value::zero())
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
    pub fn finalize<F: FeeAlgorithm<tx::Transaction<Address, Extra>>>(
        mut self,
        fee_algorithm: F,
        policy: OutputPolicy,
    ) -> Result<(Balance, tx::Transaction<Address, Extra>), Error> {
        // calculate initial fee, maybe we can fit it without any
        // additional calculations.
        let fee = fee_algorithm
            .calculate(&self.tx)
            .ok_or(Error::MathErr(ValueError::Overflow))?;
        let pos = match self.tx.balance(fee) {
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
                    .calculate(&tx)
                    .ok_or(Error::MathErr(ValueError::Overflow))?;
                match tx.balance(fee) {
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

custom_error! {pub BuildError
    WitnessOutOfBound { index: usize, max: usize } = "Witness index {index} out of bound (max {max})",
    WitnessMismatch { index: usize } = "Invalid witness type at index {index}",
    MissingWitnessAt { index: usize } = "Missing a witness for input at index {index}",
}

fn set_witness<Address, Extra>(
    transaction: &tx::Transaction<Address, Extra>,
    witnesses: &mut Vec<Option<tx::Witness>>,
    index: usize,
    witness: tx::Witness,
) -> Result<(), BuildError> {
    if index >= witnesses.len() {
        return Err(BuildError::WitnessOutOfBound {
            index,
            max: witnesses.len(),
        });
    }

    match (transaction.inputs[index].get_type(), &witness) {
        (tx::InputType::Utxo, tx::Witness::OldUtxo(_, _)) => (),
        (tx::InputType::Utxo, tx::Witness::Utxo(_)) => (),
        (tx::InputType::Account, tx::Witness::Account(_)) => (),
        (_, _) => return Err(BuildError::WitnessMismatch { index }),
    };

    witnesses[index] = Some(witness);
    Ok(())
}

fn get_full_witnesses(witnesses: Vec<Option<tx::Witness>>) -> Result<Vec<tx::Witness>, BuildError> {
    let mut v = Vec::new();
    for (i, w) in witnesses.iter().enumerate() {
        match w {
            None => return Err(BuildError::MissingWitnessAt { index: i }),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fee::LinearFee;
    use crate::transaction::{Input, NoExtra, INPUT_PTR_SIZE};
    use chain_addr::Address;
    use quickcheck::{Arbitrary, Gen, TestResult};
    use quickcheck_macros::quickcheck;
    use std::iter;

    #[quickcheck]
    fn tx_builder_never_creates_unbalanced_tx(
        inputs: ArbitraryInputs,
        outputs: ArbitraryOutputs,
        fee: LinearFee,
    ) -> TestResult {
        let builder = build_builder(&inputs, &outputs);
        let fee_value = fee.calculate(&builder.tx).unwrap();

        let result = builder.finalize(fee, OutputPolicy::Forget);

        let expected_balance_res = expected_balance(&inputs, &outputs, fee_value);
        match (expected_balance_res, result) {
            (Ok(expected_balance), Ok((builder_balance, tx))) => {
                let result = validate_builder_balance(expected_balance, builder_balance);
                if result.is_failure() {
                    return result;
                }
                validate_tx_balance(expected_balance, fee_value, tx)
            }
            (Ok(_), Err(_)) => TestResult::error("Builder should not fail"),
            (Err(_), Ok(_)) => TestResult::error("Builder should fail"),
            (Err(_), Err(_)) => TestResult::passed(),
        }
    }

    fn build_builder(
        inputs: &ArbitraryInputs,
        outputs: &ArbitraryOutputs,
    ) -> TransactionBuilder<Address, NoExtra> {
        let mut builder = TransactionBuilder::new();
        for input in &inputs.0 {
            builder.add_input(input)
        }
        for (address, value) in outputs.0.iter().cloned() {
            builder.add_output(address, value)
        }
        builder
    }

    fn expected_balance(
        inputs: &ArbitraryInputs,
        outputs: &ArbitraryOutputs,
        fee: Value,
    ) -> Result<Value, ValueError> {
        let input_sum = Value::sum(inputs.0.iter().map(|input| input.value)).unwrap();
        let output_sum = Value::sum(outputs.0.iter().map(|output| output.1)).unwrap();
        (input_sum - output_sum).and_then(|balance| balance - fee)
    }

    fn validate_builder_balance(expected: Value, balance: Balance) -> TestResult {
        let actual = match balance {
            Balance::Positive(value) => value,
            Balance::Zero => Value::zero(),
            Balance::Negative(_) => return TestResult::error("Negative balance in builder"),
        };
        if actual != expected {
            TestResult::error(format!(
                "Builder balance value is {}, but should be {}",
                actual, expected
            ))
        } else {
            TestResult::passed()
        }
    }

    fn validate_tx_balance(
        expected: Value,
        fee: Value,
        tx: tx::Transaction<Address, NoExtra>,
    ) -> TestResult {
        let actual = match tx.balance(fee) {
            Ok(Balance::Positive(value)) => value,
            Ok(Balance::Zero) => Value::zero(),
            Ok(Balance::Negative(_)) => return TestResult::error("Negative balance in tx"),
            Err(_) => return TestResult::error("Failed to calculate tx balance"),
        };
        if actual != expected {
            TestResult::error(format!(
                "Tx balance value is {}, but should be {}",
                actual, expected
            ))
        } else {
            TestResult::passed()
        }
    }

    #[derive(Clone, Debug)]
    struct ArbitraryInputs(Vec<Input>);

    impl Arbitrary for ArbitraryInputs {
        fn arbitrary<G: Gen>(gen: &mut G) -> Self {
            let value = u64::arbitrary(gen);
            let count = u8::arbitrary(gen);
            let inputs = split_value(gen, value, count as u16)
                .into_iter()
                .map(|value| arbitrary_input(gen, value))
                .collect();
            ArbitraryInputs(inputs)
        }
    }

    #[derive(Clone, Debug)]
    struct ArbitraryOutputs(Vec<(Address, Value)>);

    impl Arbitrary for ArbitraryOutputs {
        fn arbitrary<G: Gen>(gen: &mut G) -> Self {
            let value = u64::arbitrary(gen);
            let count = u8::arbitrary(gen);
            let outputs = split_value(gen, value, count as u16)
                .into_iter()
                .map(|value| (Address::arbitrary(gen), Value(value)))
                .collect();
            ArbitraryOutputs(outputs)
        }
    }

    fn arbitrary_input(gen: &mut impl Gen, value: u64) -> Input {
        let mut input_ptr = [0; INPUT_PTR_SIZE];
        gen.fill_bytes(&mut input_ptr);
        Input {
            index_or_account: u8::arbitrary(gen),
            value: Value(value),
            input_ptr,
        }
    }

    fn split_value(gen: &mut impl Gen, value: u64, parts: u16) -> Vec<u64> {
        let mut in_values: Vec<_> = iter::once(0)
            .chain(iter::repeat_with(|| arbitrary_range(gen, value)))
            .take(parts as usize)
            .chain(iter::once(value))
            .collect();
        in_values.sort();
        in_values.windows(2).map(|pair| pair[1] - pair[0]).collect()
    }

    fn arbitrary_range(gen: &mut impl Gen, range: u64) -> u64 {
        u64::arbitrary(gen) % (range + 1)
    }

    #[quickcheck]
    fn split_value_splits_whole_value(split_value: ArbitrarySplitValue) -> () {
        assert_eq!(
            split_value.parts,
            split_value.split.len(),
            "Invalid split length"
        );
        assert_eq!(
            split_value.value,
            split_value.split.iter().sum(),
            "Invalid split sum"
        );
    }

    #[derive(Clone, Debug)]
    struct ArbitrarySplitValue {
        value: u64,
        parts: usize,
        split: Vec<u64>,
    }

    impl Arbitrary for ArbitrarySplitValue {
        fn arbitrary<G: Gen>(gen: &mut G) -> Self {
            let value = u64::arbitrary(gen);
            let parts = u16::arbitrary(gen);
            let split = split_value(gen, value, parts);
            let value = match parts {
                0 => 0,
                _ => value,
            };
            ArbitrarySplitValue {
                value,
                parts: parts as usize,
                split,
            }
        }
    }
}

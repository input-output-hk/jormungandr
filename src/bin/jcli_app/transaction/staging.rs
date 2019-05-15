use chain_addr::Address;
use chain_impl_mockchain::{
    self as chain,
    fee::FeeAlgorithm,
    transaction::{NoExtra, Transaction},
    value::Value,
};
use jcli_app::utils::io;
use jormungandr_utils;
use serde::{Deserialize, Serialize};
use std::path::Path;

const INPUT_PTR_SIZE: usize = 32;

#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
pub enum StagingKind {
    Balancing,
    Finalizing,
    Sealed,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Input {
    pub index_or_account: u8,
    #[serde(
        serialize_with = "jormungandr_utils::serde::value::serialize",
        deserialize_with = "jormungandr_utils::serde::value::deserialize"
    )]
    pub value: Value,
    pub input_ptr: [u8; INPUT_PTR_SIZE],
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Output {
    #[serde(
        serialize_with = "jormungandr_utils::serde::address::serialize",
        deserialize_with = "jormungandr_utils::serde::address::deserialize"
    )]
    pub address: Address,
    #[serde(
        serialize_with = "jormungandr_utils::serde::value::serialize",
        deserialize_with = "jormungandr_utils::serde::value::deserialize"
    )]
    pub value: Value,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Witness {
    #[serde(
        serialize_with = "jormungandr_utils::serde::witness::serialize",
        deserialize_with = "jormungandr_utils::serde::witness::deserialize"
    )]
    pub witness: chain::transaction::Witness,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Staging {
    pub kind: StagingKind,

    pub inputs: Vec<Input>,

    pub outputs: Vec<Output>,

    pub witnesses: Vec<Witness>,
}

custom_error! {pub StagingError
    CannotLoad { source: bincode::Error } = "cannot load encoded staging transaction",
    CannotAddInput { kind: StagingKind } = "cannot add input in the {kind} transaction",
    CannotAddOutput { kind: StagingKind } = "cannot add output in the {kind} transaction",
    CannotAddWitness { kind: StagingKind } = "cannot add witness in the {kind} transaction",
    CannotAddWitnessTooManyWitnesses = "cannot add anymore witnesses",
    CannotFinalize { kind: StagingKind } = "cannot finalize {kind} transaction",
    CannotSeal { kind: StagingKind } = "cannot seal {kind} transaction",
    CannotSealNotEnoughWitnesses = "cannot seal, not enough witnesses",
    CannotSealFinalizerError { error: chain::txbuilder::BuildError } = "cannot seal: {error}",
    CannotFinalizeTransaction { source: chain::txbuilder::Error } = "Cannot finalize the transaction",
    CannotGetMessage { kind: StagingKind } = "cannot get message from {kind} transaction",
    CannotGetMessageFinalizerError { error: chain::txbuilder::BuildError } = "cannot get message from transaction: {error}",
}

impl std::fmt::Display for StagingKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            StagingKind::Balancing => write!(f, "balancing"),
            StagingKind::Finalizing => write!(f, "finalizing"),
            StagingKind::Sealed => write!(f, "sealed"),
        }
    }
}

impl Staging {
    pub fn new() -> Self {
        Staging {
            kind: StagingKind::Balancing,
            inputs: Vec::new(),
            outputs: Vec::new(),
            witnesses: Vec::new(),
        }
    }

    pub fn load<P: AsRef<Path>>(path: &Option<P>) -> Result<Self, StagingError> {
        let mut file = io::open_file_read(path).unwrap();
        Ok(bincode::deserialize_from(&mut file)?)
    }

    pub fn store<P: AsRef<Path>>(&self, path: &Option<P>) -> Result<(), StagingError> {
        let file = io::open_file_write(path).unwrap();
        Ok(bincode::serialize_into(file, self)?)
    }

    pub fn add_input(&mut self, input: chain::transaction::Input) -> Result<(), StagingError> {
        if self.kind != StagingKind::Balancing {
            return Err(StagingError::CannotAddInput { kind: self.kind });
        }

        Ok(self.inputs.push(Input {
            index_or_account: input.index_or_account,
            value: input.value,
            input_ptr: input.input_ptr,
        }))
    }

    pub fn add_output(
        &mut self,
        output: chain::transaction::Output<Address>,
    ) -> Result<(), StagingError> {
        if self.kind != StagingKind::Balancing {
            return Err(StagingError::CannotAddOutput { kind: self.kind });
        }

        Ok(self.outputs.push(Output {
            address: output.address,
            value: output.value,
        }))
    }

    pub fn add_witness(
        &mut self,
        witness: chain::transaction::Witness,
    ) -> Result<(), StagingError> {
        if self.kind != StagingKind::Finalizing {
            return Err(StagingError::CannotAddWitness { kind: self.kind });
        }

        if self.inputs.len() <= self.witnesses.len() {
            return Err(StagingError::CannotAddWitnessTooManyWitnesses);
        }

        Ok(self.witnesses.push(Witness { witness }))
    }

    pub fn finalize<FA>(
        &mut self,
        fee_algorithm: FA,
        output_policy: chain::txbuilder::OutputPolicy,
    ) -> Result<chain::transaction::Balance, StagingError>
    where
        FA: FeeAlgorithm<Transaction<Address, NoExtra>>,
    {
        if self.kind != StagingKind::Balancing {
            return Err(StagingError::CannotFinalize { kind: self.kind });
        }
        let builder = self.builder();

        let (balance, tx) = builder.finalize(fee_algorithm, output_policy)?;

        self.inputs = tx
            .inputs
            .into_iter()
            .map(|input| Input {
                index_or_account: input.index_or_account,
                value: input.value,
                input_ptr: input.input_ptr,
            })
            .collect();
        self.outputs = tx
            .outputs
            .into_iter()
            .map(|output| Output {
                address: output.address,
                value: output.value,
            })
            .collect();

        self.kind = StagingKind::Finalizing;

        Ok(balance)
    }

    pub fn seal(&mut self) -> Result<(), StagingError> {
        if self.kind != StagingKind::Finalizing {
            return Err(StagingError::CannotSeal { kind: self.kind });
        }

        if self.inputs.len() != self.witnesses.len() {
            return Err(StagingError::CannotSealNotEnoughWitnesses);
        }

        Ok(self.kind = StagingKind::Sealed)
    }

    pub fn message(&self) -> Result<chain::message::Message, StagingError> {
        if self.kind != StagingKind::Sealed {
            return Err(StagingError::CannotGetMessage { kind: self.kind });
        }

        let transaction = self.finalizer()?;

        let result = transaction
            .build()
            .map_err(|error| StagingError::CannotGetMessageFinalizerError { error })?;

        match result {
            chain::txbuilder::GeneratedTransaction::Type1(auth) => {
                Ok(chain::message::Message::Transaction(auth))
            }
            _ => unreachable!(),
        }
    }

    pub fn transaction(
        &self,
    ) -> chain::transaction::Transaction<Address, chain::transaction::NoExtra> {
        chain::transaction::Transaction {
            inputs: self.inputs(),
            outputs: self.outputs(),
            extra: chain::transaction::NoExtra,
        }
    }

    pub fn builder(
        &self,
    ) -> chain::txbuilder::TransactionBuilder<Address, chain::transaction::NoExtra> {
        chain::txbuilder::TransactionBuilder::from(self.transaction())
    }

    pub fn finalizer(&self) -> Result<chain::txbuilder::TransactionFinalizer, StagingError> {
        let transaction = self.transaction();
        let mut finalizer = chain::txbuilder::TransactionFinalizer::new_trans(transaction);

        for (index, witness) in self.witnesses.iter().enumerate() {
            finalizer
                .set_witness(index, witness.witness.clone())
                .map_err(|error| StagingError::CannotSealFinalizerError { error })?;
        }

        Ok(finalizer)
    }

    pub fn inputs(&self) -> Vec<chain::transaction::Input> {
        self.inputs
            .iter()
            .map(|input| chain::transaction::Input {
                index_or_account: input.index_or_account,
                value: input.value,
                input_ptr: input.input_ptr,
            })
            .collect()
    }

    pub fn outputs(&self) -> Vec<chain::transaction::Output<Address>> {
        self.outputs
            .iter()
            .map(|output| chain::transaction::Output {
                address: output.address.clone(),
                value: output.value,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use chain_impl_mockchain as chain;
    use chain_impl_mockchain::key::Hash;
    use std::str::FromStr;

    #[test]
    pub fn test_initial_stage_is_balancing() {
        let staging = Staging::new();
        let expected_kind = StagingKind::Balancing;
        assert_eq!(
            staging.kind, expected_kind,
            "'initial staging kind should be {}",
            expected_kind
        );
    }

    #[test]
    pub fn test_cannot_add_input_when_stage_is_finalizing() {
        let hash =
            Hash::from_str("c355a02d3b5337ad0e5f5940582675229f25bc03e7feebc3aa929738e1fec35e")
                .unwrap();
        let incorrect_stage = StagingKind::Finalizing;

        let mut staging = Staging::new();
        staging.kind = incorrect_stage.clone();

        let mut input_ptr = [0u8; INPUT_PTR_SIZE];
        input_ptr.clone_from_slice(hash.as_ref());

        let result = staging.add_input(chain::transaction::Input {
            input_ptr: input_ptr,
            index_or_account: 0,
            value: Value(200),
        });

        assert!(
            result.is_err(),
            "add_input message should throw exception when adding inputs while in {:?} stage",
            &incorrect_stage
        );
    }
}

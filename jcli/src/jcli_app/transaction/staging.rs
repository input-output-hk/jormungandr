use chain_addr::Address;
use chain_impl_mockchain::{
    self as chain,
    fee::FeeAlgorithm,
    fragment::Fragment,
    transaction::{NoExtra, Output, Transaction, TransactionSignDataHash},
    txbuilder,
    value::Value,
};
use jcli_app::transaction::Error;
use jcli_app::utils::error::CustomErrorFiller;
use jcli_app::utils::io;
use jormungandr_lib::interfaces;
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
struct Input {
    index_or_account: u8,
    value: interfaces::Value,
    input_ptr: [u8; INPUT_PTR_SIZE],
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Staging {
    kind: StagingKind,
    inputs: Vec<Input>,
    outputs: Vec<interfaces::TransactionOutput>,
    witnesses: Vec<interfaces::TransactionWitness>,
    extra: Option<interfaces::Certificate>,
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
            extra: None,
        }
    }

    pub fn load<P: AsRef<Path>>(path: &Option<P>) -> Result<Self, Error> {
        let file = io::open_file_read(path).map_err(|source| Error::StagingFileOpenFailed {
            source,
            path: io::path_to_path_buf(path),
        })?;
        bincode::deserialize_from(file).map_err(|source| Error::StagingFileReadFailed {
            source: *source,
            path: io::path_to_path_buf(path),
        })
    }

    pub fn store<P: AsRef<Path>>(&self, path: &Option<P>) -> Result<(), Error> {
        let file = io::open_file_write(path).map_err(|source| Error::StagingFileOpenFailed {
            source,
            path: io::path_to_path_buf(path),
        })?;
        bincode::serialize_into(file, self).map_err(|source| Error::StagingFileWriteFailed {
            source: *source,
            path: io::path_to_path_buf(path),
        })
    }

    pub fn add_input(&mut self, input: chain::transaction::Input) -> Result<(), Error> {
        if self.kind != StagingKind::Balancing {
            return Err(Error::TxKindToAddInputInvalid { kind: self.kind });
        }

        Ok(self.inputs.push(Input {
            index_or_account: input.index_or_account,
            value: input.value.into(),
            input_ptr: input.input_ptr,
        }))
    }

    pub fn add_output(&mut self, output: Output<Address>) -> Result<(), Error> {
        if self.kind != StagingKind::Balancing {
            return Err(Error::TxKindToAddOutputInvalid { kind: self.kind });
        }

        Ok(self.outputs.push(output.into()))
    }

    pub fn add_witness(&mut self, witness: chain::transaction::Witness) -> Result<(), Error> {
        if self.kind != StagingKind::Finalizing {
            return Err(Error::TxKindToAddWitnessInvalid { kind: self.kind });
        }

        if self.inputs.len() <= self.witnesses.len() {
            return Err(Error::TooManyWitnessesToAddWitness {
                actual: self.witnesses.len(),
                max: self.inputs.len(),
            });
        }

        Ok(self.witnesses.push(witness.into()))
    }

    pub fn set_extra(&mut self, extra: chain::certificate::Certificate) -> Result<(), Error> {
        match self.kind {
            StagingKind::Balancing => Ok(self.extra = Some(extra.into())),
            kind => Err(Error::TxKindToAddExtraInvalid { kind }),
        }
    }

    pub fn witness_count(&self) -> usize {
        self.witnesses.len()
    }

    pub fn staging_kind_name(&self) -> String {
        self.kind.to_string()
    }

    fn update_tx<Extra>(&mut self, tx: Transaction<Address, Extra>) {
        self.inputs = tx
            .inputs
            .into_iter()
            .map(|input| Input {
                index_or_account: input.index_or_account,
                value: input.value.into(),
                input_ptr: input.input_ptr,
            })
            .collect();
        self.outputs = tx
            .outputs
            .into_iter()
            .map(interfaces::TransactionOutput::from)
            .collect();
    }

    pub fn finalize<FA>(
        &mut self,
        fee_algorithm: FA,
        output_policy: chain::txbuilder::OutputPolicy,
    ) -> Result<chain::transaction::Balance, Error>
    where
        FA: FeeAlgorithm<Transaction<Address, NoExtra>>
            + FeeAlgorithm<Transaction<Address, chain::certificate::Certificate>>,
    {
        if self.kind != StagingKind::Balancing {
            return Err(Error::TxKindToFinalizeInvalid { kind: self.kind });
        }

        let balance = if let Some(certificate) = self.extra.clone() {
            let tx = self.transaction_with_extra(&certificate);
            let builder = txbuilder::TransactionBuilder::from(tx);

            let (balance, tx) = builder.finalize(fee_algorithm, output_policy)?;

            self.update_tx(tx);

            balance
        } else {
            let tx = self.transaction();
            let builder = txbuilder::TransactionBuilder::from(tx);
            let (balance, tx) = builder.finalize(fee_algorithm, output_policy)?;

            self.update_tx(tx);

            balance
        };

        self.kind = StagingKind::Finalizing;

        Ok(balance)
    }

    pub fn seal(&mut self) -> Result<(), Error> {
        if self.kind != StagingKind::Finalizing {
            return Err(Error::TxKindToSealInvalid { kind: self.kind });
        }

        if self.inputs.len() != self.witnesses.len() {
            return Err(Error::WitnessCountToSealInvalid {
                actual: self.witnesses.len(),
                expected: self.inputs.len(),
            });
        }

        Ok(self.kind = StagingKind::Sealed)
    }

    pub fn message(&self) -> Result<Fragment, Error> {
        if self.kind != StagingKind::Sealed {
            Err(Error::TxKindToGetMessageInvalid { kind: self.kind })?
        }

        let transaction = self.finalizer()?;

        let result = transaction
            .build()
            .map_err(|source| Error::GeneratedTxBuildingFailed {
                source,
                filler: CustomErrorFiller,
            })?;

        match result {
            chain::txbuilder::GeneratedTransaction::Type1(auth) => Ok(Fragment::Transaction(auth)),
            chain::txbuilder::GeneratedTransaction::Type2(auth) => Ok(Fragment::Certificate(auth)),
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

    fn transaction_with_extra(
        &self,
        certificate: &interfaces::Certificate,
    ) -> chain::transaction::Transaction<Address, chain::certificate::Certificate> {
        chain::transaction::Transaction {
            inputs: self.inputs(),
            outputs: self.outputs(),
            extra: certificate.clone().into(),
        }
    }

    pub fn builder(
        &self,
    ) -> chain::txbuilder::TransactionBuilder<Address, chain::transaction::NoExtra> {
        chain::txbuilder::TransactionBuilder::from(self.transaction())
    }

    pub fn id(&self) -> TransactionSignDataHash {
        if let Some(extra) = &self.extra {
            self.transaction_with_extra(&extra).hash()
        } else {
            self.transaction().hash()
        }
    }

    pub fn fees<FA>(&self, fee_algorithm: FA) -> Result<Value, Error>
    where
        FA: FeeAlgorithm<Transaction<Address, NoExtra>>
            + FeeAlgorithm<Transaction<Address, chain::certificate::Certificate>>,
    {
        if let Some(certificate) = &self.extra {
            let tx = self.transaction_with_extra(certificate);
            let builder = txbuilder::TransactionBuilder::from(tx);
            Ok(builder.estimate_fee(fee_algorithm)?)
        } else {
            let tx = self.transaction();
            let builder = txbuilder::TransactionBuilder::from(tx);
            Ok(builder.estimate_fee(fee_algorithm)?)
        }
    }

    pub fn balance<FA>(&self, fee_algorithm: FA) -> Result<chain::transaction::Balance, Error>
    where
        FA: FeeAlgorithm<Transaction<Address, NoExtra>>
            + FeeAlgorithm<Transaction<Address, chain::certificate::Certificate>>,
    {
        if let Some(certificate) = &self.extra {
            let tx = self.transaction_with_extra(certificate);
            let builder = txbuilder::TransactionBuilder::from(tx);
            Ok(builder.get_balance(fee_algorithm)?)
        } else {
            let tx = self.transaction();
            let builder = txbuilder::TransactionBuilder::from(tx);
            Ok(builder.get_balance(fee_algorithm)?)
        }
    }

    pub fn finalizer(&self) -> Result<chain::txbuilder::TransactionFinalizer, Error> {
        let mut finalizer = if let Some(certificate) = &self.extra {
            let transaction = self.transaction_with_extra(certificate);
            chain::txbuilder::TransactionFinalizer::new_cert(transaction)
        } else {
            let transaction = self.transaction();
            chain::txbuilder::TransactionFinalizer::new_trans(transaction)
        };

        for (index, witness) in self.witnesses.iter().enumerate() {
            finalizer
                .set_witness(index, witness.clone().into())
                .map_err(|source| Error::AddingWitnessToFinalizedTxFailed {
                    source,
                    filler: CustomErrorFiller,
                })?;
        }

        Ok(finalizer)
    }

    pub fn inputs(&self) -> Vec<chain::transaction::Input> {
        self.inputs
            .iter()
            .map(|input| chain::transaction::Input {
                index_or_account: input.index_or_account,
                value: input.value.into(),
                input_ptr: input.input_ptr,
            })
            .collect()
    }

    pub fn outputs(&self) -> Vec<Output<Address>> {
        self.outputs.iter().cloned().map(Output::from).collect()
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

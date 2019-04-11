use chain_core::mempack::{ReadBuf, ReadError, Readable as _};
use chain_impl_mockchain::transaction::Witness;
use jcli_app::{transaction::common, utils::io};
use std::path::PathBuf;
use structopt::StructOpt;

custom_error! {pub AddWitnessError
    ReadTransaction { error: common::CommonError } = "cannot read the transaction: {error}",
    WriteTransaction { error: common::CommonError } = "cannot save changes of the transaction: {error}",
    ReadWitness { error: common::CommonError } = "cannot read witness: {error}",
    DeserializeWitness { source: ReadError } = "Invalid witness",
    ExceedsInput { len: usize } = "Already all needed witnesses ({len})",
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct AddWitness {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,

    pub witness: PathBuf,
}

impl AddWitness {
    pub fn exec(self) -> Result<(), AddWitnessError> {
        let mut transaction = self
            .common
            .load_auth_transaction()
            .map_err(|error| AddWitnessError::ReadTransaction { error })?;

        let witness = self.witness()?;

        if transaction.witnesses.len() == transaction.transaction.inputs.len() {
            return Err(AddWitnessError::ExceedsInput {
                len: transaction.transaction.inputs.len(),
            });
        }
        transaction.witnesses.push(witness);

        Ok(self
            .common
            .write_auth_transaction(&transaction)
            .map_err(|error| AddWitnessError::WriteTransaction { error })?)
    }

    fn witness(&self) -> Result<Witness, AddWitnessError> {
        let reader = io::open_file_read(&Some(self.witness.clone()));
        let bytes = common::read_bytes(reader, "witness")
            .map_err(|error| AddWitnessError::ReadWitness { error })?;

        Ok(Witness::read(&mut ReadBuf::from(&bytes))?)
    }
}

use chain_core::mempack::{ReadBuf, ReadError, Readable as _};
use chain_impl_mockchain::transaction::Witness;
use jcli_app::{
    transaction::{common, staging::StagingError},
    utils::io,
};
use std::path::PathBuf;
use structopt::StructOpt;

custom_error! {pub AddWitnessError
    ReadTransaction { error: StagingError } = "cannot read the transaction: {error}",
    WriteTransaction { error: StagingError } = "cannot save changes of the transaction: {error}",
    ReadWitness { error: common::CommonError } = "cannot read witness: {error}",
    AddWitness { source: StagingError } = "cannot add witness",
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
            .load()
            .map_err(|error| AddWitnessError::ReadTransaction { error })?;

        let witness = self.witness()?;

        transaction.add_witness(witness)?;

        Ok(self
            .common
            .store(&transaction)
            .map_err(|error| AddWitnessError::WriteTransaction { error })?)
    }

    fn witness(&self) -> Result<Witness, AddWitnessError> {
        let reader = io::open_file_read(&Some(self.witness.clone())).unwrap();
        let bytes = common::read_bytes(reader, "witness")
            .map_err(|error| AddWitnessError::ReadWitness { error })?;

        Ok(Witness::read(&mut ReadBuf::from(&bytes))?)
    }
}

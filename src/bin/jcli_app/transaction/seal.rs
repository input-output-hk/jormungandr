use jcli_app::transaction::{common, staging::StagingError};
use structopt::StructOpt;

custom_error! {pub SealError
    ReadTransaction { error: StagingError } = "cannot read the transaction: {error}",
    WriteTransaction { error: StagingError } = "cannot save changes of the transaction: {error}",
    CannotSeal { error: StagingError } = "cannot seal the transaction: {error}",
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Seal {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,
}

impl Seal {
    pub fn exec(self) -> Result<(), SealError> {
        let mut transaction = self
            .common
            .load()
            .map_err(|error| SealError::ReadTransaction { error })?;

        transaction
            .seal()
            .map_err(|error| SealError::CannotSeal { error })?;

        Ok(self
            .common
            .store(&transaction)
            .map_err(|error| SealError::WriteTransaction { error })?)
    }
}

use chain_impl_mockchain::{
    certificate::{Certificate},
};
use jcli_app::transaction::{common, staging::StagingError};
use jormungandr_utils::{certificate};
use structopt::StructOpt;

custom_error! {pub AddCertificateError
    StagingError { source: StagingError } = "Add certificate operation failed",
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct AddCertificate {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,

    /// the value
    #[structopt(name = "VALUE", parse(try_from_str = "certificate::deserialize_from_bech32"))]
    pub certificate: Certificate,
}

impl AddCertificate {
    pub fn exec(self) -> Result<(), AddCertificateError> {
        let mut transaction = self.common.load()?;

        transaction.set_extra(self.certificate)?;

        Ok(self.common.store(&transaction)?)
    }
}

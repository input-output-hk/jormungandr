use crate::transaction::{common, Error};
use jormungandr_lib::interfaces::Certificate;
#[cfg(feature = "structopt")]
use structopt::StructOpt;

#[cfg_attr(
    feature = "structopt",
    derive(StructOpt),
    structopt(rename_all = "kebab-case")
)]
pub struct AddCertificate {
    #[cfg_attr(feature = "structopt", structopt(flatten))]
    pub common: common::CommonTransaction,

    /// bech32-encoded certificate
    pub certificate: Certificate,
}

impl AddCertificate {
    pub fn exec(self) -> Result<(), Error> {
        let mut transaction = self.common.load()?;
        transaction.set_extra(self.certificate.into())?;
        self.common.store(&transaction)
    }
}

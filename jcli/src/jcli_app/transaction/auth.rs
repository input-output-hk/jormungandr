use jcli_app::transaction::{common, Error};
use jormungandr_lib::interfaces::Certificate;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Auth {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,
    // the value
    //#[structopt(name = "VALUE", parse(try_from_str))]
    //pub certificate: Certificate,
}

impl Auth {
    pub fn exec(self) -> Result<(), Error> {
        let mut transaction = self.common.load()?;

        transaction.set_auth()?; //self.certificate.into())?;

        self.common.store(&transaction)
    }
}

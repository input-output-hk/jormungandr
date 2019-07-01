use chain_impl_mockchain::certificate::{Certificate, CertificateContent};
use jcli_app::certificate::{self, Error};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct GetStakePoolId {
    /// read the certificate from
    pub input: Option<PathBuf>,
    /// write the certificate too
    pub output: Option<PathBuf>,
}

impl GetStakePoolId {
    pub fn exec(self) -> Result<(), Error> {
        let cert : Certificate = certificate::read_cert(self.input)?.into();
        match cert.content {
            CertificateContent::StakePoolRegistration(stake_pool_info) => {
                certificate::write_output(self.output, stake_pool_info.to_id())
            }
            _ => Err(Error::NotStakePoolRegistration),
        }
    }
}

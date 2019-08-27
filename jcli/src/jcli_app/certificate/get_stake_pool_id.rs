use chain_impl_mockchain::certificate::{Certificate};
use jcli_app::certificate::{Error, read_cert, write_output};
use jormungandr_lib::interfaces::{Certificate as CertificateType};
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
        let cert: CertificateType = read_cert(self.input)?.into();
        match cert.0 {
            Certificate::PoolRegistration(stake_pool_info) => {
                write_output(self.output, stake_pool_info.to_id())
            }
            _ => Err(Error::NotStakePoolRegistration),
        }
    }
}

use chain_impl_mockchain::certificate::Certificate;
use jcli_app::certificate::{read_cert_or_signed_cert, write_output, Error};
use jormungandr_lib::interfaces::Certificate as CertificateType;
use std::ops::Deref;
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
        let cert: CertificateType =
            read_cert_or_signed_cert(self.input.as_ref().map(|x| x.deref()))?.into();
        match cert.0 {
            Certificate::PoolRegistration(stake_pool_info) => write_output(
                self.output.as_ref().map(|x| x.deref()),
                stake_pool_info.to_id(),
            ),
            _ => Err(Error::NotStakePoolRegistration),
        }
    }
}

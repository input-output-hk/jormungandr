use crate::certificate::{read_cert_or_signed_cert, write_output, Error};
use chain_impl_mockchain::certificate::Certificate;
use jormungandr_lib::interfaces::Certificate as CertificateType;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct GetStakePoolId {
    /// file to read the certificate from (defaults to stdin)
    #[structopt(long, parse(from_os_str), value_name = "PATH")]
    pub input: Option<PathBuf>,
    /// file to write the output to (defaults to stdout)
    #[structopt(long, parse(from_os_str), value_name = "PATH")]
    pub output: Option<PathBuf>,
}

impl GetStakePoolId {
    pub fn exec(self) -> Result<(), Error> {
        let cert: CertificateType = read_cert_or_signed_cert(self.input.as_deref())?;
        match cert.0 {
            Certificate::PoolRegistration(stake_pool_info) => {
                write_output(self.output.as_deref(), stake_pool_info.to_id())
            }
            Certificate::PoolRetirement(stake_pool_info) => {
                write_output(self.output.as_deref(), stake_pool_info.pool_id)
            }
            _ => Err(Error::NotStakePoolRegistration),
        }
    }
}

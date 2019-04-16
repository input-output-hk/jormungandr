use chain_impl_mockchain::certificate::CertificateContent;
use jcli_app::utils::io;
use jormungandr_utils::certificate as cert_utils;
use std::path::PathBuf;
use structopt::StructOpt;

custom_error! {pub Error
    Encoding { source: cert_utils::Error } = "Invalid certificate",
    CryptoEncoding { source: chain_crypto::bech32::Error } = "Invalid private key",
    Io { source: std::io::Error } = "I/O Error",
    NotStakePoolRegistration = "Invalid certificate, expecting a stake pool registration",
}

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
        let mut input = io::open_file_read(&self.input);
        let mut input_str = String::new();
        input.read_to_string(&mut input_str)?;
        let cert = cert_utils::deserialize_from_bech32(&input_str.trim())?;

        match cert.content {
            CertificateContent::StakePoolRegistration(s) => {
                let id = s.to_id();
                let mut f = io::open_file_write(&self.output);
                writeln!(f, "{}", id)?;
                Ok(())
            }
            _ => Err(Error::NotStakePoolRegistration),
        }
    }
}

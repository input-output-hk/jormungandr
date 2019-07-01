use chain_impl_mockchain::certificate::{Certificate, CertificateContent};
use jcli_app::certificate::{self, Error};
use jcli_app::utils::key_parser::parse_ed25519_secret_key;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Sign {
    /// path to the file with the signing key
    pub signing_key: PathBuf,
    /// get the certificate to sign from the given file. If no file
    /// provided, it will be read from the standard input
    pub input: Option<PathBuf>,
    /// write the signed certificate into the given file. If no file
    /// provided it will be written into the standard output
    pub output: Option<PathBuf>,
}

impl Sign {
    pub fn exec(self) -> Result<(), Error> {
        let mut cert: Certificate = certificate::read_cert(self.input)?.into();
        let key_str = certificate::read_input(Some(self.signing_key))?;
        let private_key = parse_ed25519_secret_key(key_str.trim())?;

        let signature = match &cert.content {
            CertificateContent::StakeDelegation(s) => s.make_certificate(&private_key),
            CertificateContent::StakePoolRegistration(s) => s.make_certificate(&private_key),
            CertificateContent::StakePoolRetirement(s) => s.make_certificate(&private_key),
        };
        cert.signatures.push(signature);
        certificate::write_cert(self.output, cert.into())
    }
}

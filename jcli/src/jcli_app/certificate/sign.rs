use chain_impl_mockchain::certificate::Certificate;
use jcli_app::certificate::{read_cert, read_input, write_cert, Error};
use jcli_app::utils::key_parser::parse_ed25519_secret_key;
use jormungandr_lib::interfaces::Certificate as CertificateType;
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
        let mut cert: CertificateType = read_cert(self.input)?.into();
        let key_str = read_input(Some(self.signing_key))?;
        let private_key = parse_ed25519_secret_key(key_str.trim())?;

        /*
        let signature = match &cert.0 {
            Certificate::StakeDelegation(s) => s.make_certificate(&private_key),
            Certificate::PoolRegistration(s) => s.make_certificate(&private_key),
            Certificate::PoolManagement(s) => s.make_certificate(&private_key),
        };
        cert.signatures.push(signature);
        */
        write_cert(self.output, cert.into())
    }
}

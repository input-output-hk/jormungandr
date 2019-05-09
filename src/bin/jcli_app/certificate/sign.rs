use chain_crypto::{bech32::Bech32, Ed25519Extended, SecretKey};
use chain_impl_mockchain::certificate::CertificateContent;
use jcli_app::certificate::{self, Error};
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
        let mut cert = certificate::read_cert(self.input)?;
        let key_str = certificate::read_input(Some(self.signing_key))?;
        let private_key = SecretKey::<Ed25519Extended>::try_from_bech32_str(key_str.trim())?;

        let signature = match &cert.content {
            CertificateContent::StakeKeyRegistration(s) => s.make_certificate(&private_key),
            CertificateContent::StakeKeyDeregistration(s) => s.make_certificate(&private_key),
            CertificateContent::StakeDelegation(s) => s.make_certificate(&private_key),
            CertificateContent::StakePoolRegistration(s) => s.make_certificate(&private_key),
            CertificateContent::StakePoolRetirement(s) => s.make_certificate(&private_key),
        };
        cert.signatures.push(signature);
        certificate::write_cert(self.output, cert)
    }
}

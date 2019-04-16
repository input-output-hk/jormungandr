use chain_crypto::{bech32, Ed25519Extended, SecretKey};
use chain_impl_mockchain::certificate::CertificateContent;
use jcli_app::utils::io;
use jormungandr_utils::certificate as cert_utils;
use std::{fs, path::PathBuf};
use structopt::StructOpt;

custom_error! {pub Error
    Encoding { source: cert_utils::Error } = "Invalid certificate",
    CryptoEncoding { source: chain_crypto::bech32::Error } = "Invalid private key",
    Io { source: std::io::Error } = "I/O Error",
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Sign {
    pub signing_key: PathBuf,
    pub input: Option<PathBuf>,
    pub output: Option<PathBuf>,
}

impl Sign {
    pub fn exec(self) -> Result<(), Error> {
        let mut input = io::open_file_read(&self.input);
        let mut input_str = String::new();
        input.read_to_string(&mut input_str)?;
        let mut cert = cert_utils::deserialize_from_bech32(&input_str.trim())?;
        let key_str = fs::read_to_string(self.signing_key)?;
        let private_key =
            <SecretKey<Ed25519Extended> as bech32::Bech32>::try_from_bech32_str(&key_str.trim())?;
        let signature = match &cert.content {
            CertificateContent::StakeKeyRegistration(s) => s.make_certificate(&private_key),
            CertificateContent::StakeKeyDeregistration(s) => s.make_certificate(&private_key),
            CertificateContent::StakeDelegation(s) => s.make_certificate(&private_key),
            CertificateContent::StakePoolRegistration(s) => s.make_certificate(&private_key),
            CertificateContent::StakePoolRetirement(s) => s.make_certificate(&private_key),
        };
        cert.signatures.push(signature);
        let bech32 = cert_utils::serialize_to_bech32(&cert)?;
        let mut f = io::open_file_write(&self.output);
        writeln!(f, "{}", bech32)?;
        Ok(())
    }
}

use chain_crypto::{Ed25519Extended, PublicKey};
use chain_impl_mockchain::certificate::{
    self, CertificateContent, StakeKeyRegistration as Registration,
};
use jcli_app::utils::io;
use jcli_app::utils::key_parser::parse_pub_key;
use jormungandr_utils::certificate as cert_utils;
use std::path::PathBuf;
use structopt::StructOpt;

custom_error! {pub Error
    Encoding { source: cert_utils::Error } = "Invalid certificate",
    Io { source: std::io::Error } = "I/O error",
}

#[derive(StructOpt)]
pub struct StakeKeyRegistration {
    /// the delegation key
    #[structopt(name = "PUBLIC_KEY", parse(from_str = "parse_pub_key"))]
    pub key: PublicKey<Ed25519Extended>,
    /// print the output signed certificate in the given file, if no file given
    /// the output will be printed in the standard output
    pub output: Option<PathBuf>,
}

impl StakeKeyRegistration {
    pub fn exec(self) -> Result<(), Error> {
        let content = Registration {
            stake_key_id: self.key.into(),
        };

        let cert = certificate::Certificate {
            content: CertificateContent::StakeKeyRegistration(content),
            signatures: vec![],
        };

        let bech32 = cert_utils::serialize_to_bech32(&cert)?;
        let mut file = io::open_file_write(&self.output);
        writeln!(file, "{}", bech32)?;
        Ok(())
    }
}

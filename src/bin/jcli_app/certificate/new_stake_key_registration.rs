use chain_crypto::{Ed25519Extended, PublicKey};
use chain_impl_mockchain::certificate::{
    Certificate, CertificateContent, StakeKeyRegistration as Registration,
};
use jcli_app::certificate::{self, Error};
use jcli_app::utils::key_parser::parse_pub_key;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct StakeKeyRegistration {
    /// the delegation key
    #[structopt(name = "PUBLIC_KEY", parse(try_from_str = "parse_pub_key"))]
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
        let cert = Certificate {
            content: CertificateContent::StakeKeyRegistration(content),
            signatures: vec![],
        };
        certificate::write_cert(self.output, cert)
    }
}

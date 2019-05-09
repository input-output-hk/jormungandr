use chain_crypto::{Ed25519Extended, PublicKey};
use chain_impl_mockchain::certificate::{self, CertificateContent, StakeDelegation as Delegation};
use jcli_app::utils::io;
use jcli_app::utils::key_parser::parse_pub_key;
use jormungandr_utils::certificate as cert_utils;
use std::io::Write;
use std::path::PathBuf;
use structopt::StructOpt;

custom_error! {pub Error
    Encoding { source: cert_utils::Error } = "Invalid certificate",
    Io { source: std::io::Error } = "I/O error",
}

#[derive(StructOpt)]
pub struct StakeDelegation {
    /// the stake pool id
    #[structopt(name = "STAKE_POOL_ID", parse(try_from_str))]
    pub pool_id: chain_crypto::Blake2b256,
    /// the delegation key
    #[structopt(name = "DELEGATION_ID", parse(try_from_str = "parse_pub_key"))]
    pub stake_id: PublicKey<Ed25519Extended>,
    /// print the output signed certificate in the given file, if no file given
    /// the output will be printed in the standard output
    pub output: Option<PathBuf>,
}

impl StakeDelegation {
    pub fn exec(self) -> Result<(), Error> {
        let content = Delegation {
            stake_key_id: self.stake_id.into(),
            pool_id: self.pool_id.into(),
        };

        let cert = certificate::Certificate {
            content: CertificateContent::StakeDelegation(content),
            signatures: vec![],
        };

        let bech32 = cert_utils::serialize_to_bech32(&cert)?;
        let mut file = io::open_file_write(&self.output).unwrap();
        writeln!(file, "{}", bech32)?;
        Ok(())
    }
}

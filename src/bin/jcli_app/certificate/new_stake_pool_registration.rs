use chain_crypto::{Curve25519_2HashDH, Ed25519Extended, FakeMMM, PublicKey};
use chain_impl_mockchain::{
    certificate::{self, CertificateContent},
    leadership::genesis::GenesisPraosLeader,
    stake::{StakeKeyId, StakePoolInfo},
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

#[derive(Debug, StructOpt)]
pub struct StakePoolRegistration {
    /// serial code for the stake pool certificate
    #[structopt(long = "serial", name = "SERIAL")]
    pub serial: u128,
    /// public key of the owner(s)
    #[structopt(
        long = "owner",
        name = "PUBLIC_KEY",
        parse(try_from_str = "parse_pub_key")
    )]
    pub owners: Vec<PublicKey<Ed25519Extended>>,
    /// Public key of the block signing key
    #[structopt(
        long = "kes-key",
        name = "KES_KEY",
        parse(try_from_str = "parse_pub_key")
    )]
    pub kes_key: PublicKey<FakeMMM>,
    /// public key of the VRF key
    #[structopt(
        long = "vrf-key",
        name = "VRF_KEY",
        parse(try_from_str = "parse_pub_key")
    )]
    pub vrf_key: PublicKey<Curve25519_2HashDH>,
    /// print the output signed certificate in the given file, if no file given
    /// the output will be printed in the standard output
    pub output: Option<PathBuf>,
}

impl StakePoolRegistration {
    pub fn exec(self) -> Result<(), Error> {
        let content = StakePoolInfo {
            serial: self.serial,
            owners: self
                .owners
                .into_iter()
                .map(|key| StakeKeyId::from(key))
                .collect(),
            initial_key: GenesisPraosLeader {
                kes_public_key: self.kes_key,
                vrf_public_key: self.vrf_key,
            },
        };

        let cert = certificate::Certificate {
            content: CertificateContent::StakePoolRegistration(content),
            signatures: vec![],
        };

        let bech32 = cert_utils::serialize_to_bech32(&cert)?;
        let mut file = io::open_file_write(&self.output);
        writeln!(file, "{}", bech32)?;
        Ok(())
    }
}

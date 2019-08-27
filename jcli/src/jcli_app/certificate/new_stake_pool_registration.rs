use chain_crypto::{Curve25519_2HashDH, Ed25519, PublicKey, SumEd25519_12};
use chain_impl_mockchain::{
    certificate::{Certificate, PoolRegistration},
    leadership::genesis::GenesisPraosLeader,
};
use chain_time::DurationSeconds;
use jcli_app::certificate::{write_cert, Error};
use jcli_app::utils::key_parser::parse_pub_key;
use jormungandr_lib::interfaces::Certificate as CertificateType;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct StakePoolRegistration {
    /// serial code for the stake pool certificate
    #[structopt(long = "serial", name = "SERIAL")]
    pub serial: u128,
    /// management threshold
    #[structopt(long = "management-threshold", name = "THRESHOLD")]
    pub management_threshold: u8,
    /// start validity
    #[structopt(long = "start-validity", name = "SECONDS-SINCE-START")]
    pub start_validity: u64,
    /// public key of the owner(s)
    #[structopt(
        long = "owner",
        name = "PUBLIC_KEY",
        parse(try_from_str = "parse_pub_key")
    )]
    pub owners: Vec<PublicKey<Ed25519>>,
    /// Public key of the block signing key
    #[structopt(
        long = "kes-key",
        name = "KES_KEY",
        parse(try_from_str = "parse_pub_key")
    )]
    pub kes_key: PublicKey<SumEd25519_12>,
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
        let content = PoolRegistration {
            serial: self.serial,
            owners: self.owners.clone(),
            management_threshold: self.management_threshold,
            start_validity: DurationSeconds::from(self.start_validity).into(),
            keys: GenesisPraosLeader {
                kes_public_key: self.kes_key,
                vrf_public_key: self.vrf_key,
            },
        };
        let cert = Certificate::PoolRegistration(content);
        write_cert(self.output, CertificateType(cert))
    }
}

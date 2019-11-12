use crate::jcli_app::certificate::{write_cert, Error};
use crate::jcli_app::utils::key_parser::parse_pub_key;
use chain_crypto::{Curve25519_2HashDH, Ed25519, PublicKey, SumEd25519_12};
use chain_impl_mockchain::{
    certificate::{Certificate, PoolPermissions, PoolRegistration},
    leadership::genesis::GenesisPraosLeader,
    rewards,
};
use chain_time::DurationSeconds;
use std::ops::Deref;
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
        name = "OWNER_KEY",
        parse(try_from_str = "parse_pub_key"),
        required = true
    )]
    pub owners: Vec<PublicKey<Ed25519>>,
    /// public key of the operators(s)
    #[structopt(
        long = "operator",
        name = "OPERATOR_KEY",
        parse(try_from_str = "parse_pub_key")
    )]
    pub operators: Vec<PublicKey<Ed25519>>,
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
            operators: self.operators.clone().into(),
            permissions: PoolPermissions::new(self.management_threshold),
            start_validity: DurationSeconds::from(self.start_validity).into(),
            rewards: rewards::TaxType::zero(),
            keys: GenesisPraosLeader {
                kes_public_key: self.kes_key,
                vrf_public_key: self.vrf_key,
            },
        };

        if self.management_threshold == 0 || self.management_threshold as usize > self.owners.len()
        {
            return Err(Error::ManagementThresholdInvalid {
                got: self.management_threshold as usize,
                max_expected: self.owners.len(),
            });
        };

        let cert = Certificate::PoolRegistration(content);
        write_cert(self.output.as_ref().map(|x| x.deref()), cert.into())
    }
}

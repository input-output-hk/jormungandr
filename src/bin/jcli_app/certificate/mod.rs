use chain_crypto::{bech32, Curve25519_2HashDH, Ed25519Extended, FakeMMM, PublicKey, SecretKey};
use chain_impl_mockchain::{
    certificate::{self, CertificateContent},
    leadership::genesis::GenesisPraosLeader,
    stake::{StakeKeyId, StakePoolInfo},
};
use jcli_app::utils::io;
use jcli_app::utils::key_parser::parse_pub_key;
use jormungandr_utils::certificate as cert_utils;
use std::{fs, path::PathBuf};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Certificate {
    /// Build certificate
    New(NewArgs),
    /// Sign certificate
    Sign(SignArgs),
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum NewArgs {
    StakePoolRegistration(StakePoolRegistrationArgs),
}

#[derive(StructOpt)]
pub struct StakeKeyRegistrationArgs {
    #[structopt(name = "PUBLIC_KEY")]
    pub key: String,
    #[structopt(name = "SIGNING_KEY")]
    pub private_key: PathBuf,
}

#[derive(StructOpt)]
pub struct StakeKeyDeregistrationArgs {
    #[structopt(name = "PUBLIC_KEY")]
    pub key: String,
    #[structopt(name = "SIGNING_KEY")]
    pub private_key: PathBuf,
}

#[derive(StructOpt)]
pub struct StakeDelegationArgs {
    #[structopt(name = "PUBLIC_KEY")]
    pub key: String,
    #[structopt(name = "POOL_ID")]
    pub pool_id: String,
    #[structopt(name = "SIGNING_KEY")]
    pub private_key: PathBuf,
}

#[derive(Debug, StructOpt)]
pub struct StakePoolRegistrationArgs {
    #[structopt(long = "serial", name = "SERIAL")]
    pub serial: u128,
    #[structopt(long = "owner", name = "PUBLIC_KEY", parse(from_str = "parse_pub_key"))]
    pub owners: Vec<PublicKey<Ed25519Extended>>,
    #[structopt(long = "kes-key", name = "KES_KEY", parse(from_str = "parse_pub_key"))]
    pub kes_key: PublicKey<FakeMMM>,
    #[structopt(long = "vrf-key", name = "VRF_KEY", parse(from_str = "parse_pub_key"))]
    pub vrf_key: PublicKey<Curve25519_2HashDH>,
    pub output: Option<PathBuf>,
}

impl NewArgs {
    pub fn exec(self) {
        match self {
            NewArgs::StakePoolRegistration(args) => args.exec(),
        }
    }
}

impl StakePoolRegistrationArgs {
    pub fn exec(self) {
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

        let bech32 = cert_utils::serialize_to_bech32(&cert).unwrap();
        let mut file = io::open_file_write(&self.output);
        writeln!(file, "{}", bech32).unwrap();
    }
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct SignArgs {
    pub signing_key: PathBuf,
    pub input: Option<PathBuf>,
    pub output: Option<PathBuf>,
}

impl SignArgs {
    pub fn exec(self) {
        let mut input = io::open_file_read(&self.input);
        let mut input_str = String::new();
        input
            .read_to_string(&mut input_str)
            .expect("Can't read input cert from the given input");
        let mut cert = cert_utils::deserialize_from_bech32(&input_str.trim()).unwrap();
        let key_str = fs::read_to_string(self.signing_key).unwrap();
        let private_key =
            <SecretKey<Ed25519Extended> as bech32::Bech32>::try_from_bech32_str(&key_str.trim())
                .unwrap();
        let signature = match &cert.content {
            CertificateContent::StakeKeyRegistration(s) => s.make_certificate(&private_key),
            CertificateContent::StakeKeyDeregistration(s) => s.make_certificate(&private_key),
            CertificateContent::StakeDelegation(s) => s.make_certificate(&private_key),
            CertificateContent::StakePoolRegistration(s) => s.make_certificate(&private_key),
            CertificateContent::StakePoolRetirement(s) => s.make_certificate(&private_key),
        };
        cert.signatures.push(signature);
        let bech32 = cert_utils::serialize_to_bech32(&cert).unwrap();
        let mut f = io::open_file_write(&self.output);
        writeln!(f, "{}", bech32).unwrap();
    }
}

impl Certificate {
    pub fn exec(self) {
        match self {
            Certificate::New(args) => args.exec(),
            Certificate::Sign(args) => args.exec(),
        }
    }
}

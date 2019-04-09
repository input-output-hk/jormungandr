use bech32::{u5, Bech32, FromBase32, ToBase32};
use chain_crypto::{
    AsymmetricKey, Curve25519_2HashDH, Ed25519, Ed25519Bip32, Ed25519Extended, FakeMMM,
};
use jcli_app::utils::io;
use rand::{rngs::EntropyRng, SeedableRng};
use rand_chacha::ChaChaRng;
use std::{io::Read as _, path::PathBuf};
use structopt::{clap::arg_enum, StructOpt};

#[derive(StructOpt, Debug)]
#[structopt(name = "genesis", rename_all = "kebab-case")]
pub enum Key {
    Generate(GenerateKeyArguments),
    ToPublic(ToPublicArguments),
}

#[derive(StructOpt, Debug)]
pub struct GenerateKeyArguments {
    /// Type of a private key
    ///
    /// value values are: ed25519, ed25510bip32, ed25519extended, curve25519_2hashdh
    #[structopt(long = "type")]
    pub key_type: GenPrivKeyType,

    /// output the private key in the given file or to stdout if no
    /// value is provided.
    #[structopt(name = "OUTPUT_FILE")]
    pub output: Option<PathBuf>,
}

#[derive(StructOpt, Debug)]
pub struct ToPublicArguments {
    /// the source private key to extract the public key from
    ///
    /// if no value passed, the private key will be read from the
    /// standard input
    #[structopt(long = "input")]
    pub input_key: Option<PathBuf>,

    /// output the public key in the given file or to stdout if no
    /// value is provided.
    #[structopt(name = "OUTPUT_FILE")]
    pub output: Option<PathBuf>,
}

arg_enum! {
    #[derive(StructOpt, Debug)]
    pub enum GenPrivKeyType {
        Ed25519,
        Ed25519Bip32,
        Ed25519Extended,
        FakeMMM,
        Curve25519_2HashDH,
    }
}

impl Key {
    pub fn exec(self) {
        match self {
            Key::Generate(args) => {
                let priv_key_bech32 = match args.key_type {
                    GenPrivKeyType::Ed25519 => gen_priv_key_bech32::<Ed25519>(),
                    GenPrivKeyType::Ed25519Bip32 => gen_priv_key_bech32::<Ed25519Bip32>(),
                    GenPrivKeyType::Ed25519Extended => gen_priv_key_bech32::<Ed25519Extended>(),
                    GenPrivKeyType::FakeMMM => gen_priv_key_bech32::<FakeMMM>(),
                    GenPrivKeyType::Curve25519_2HashDH => {
                        gen_priv_key_bech32::<Curve25519_2HashDH>()
                    }
                };
                let mut file = io::open_file_write(&args.output);
                writeln!(file, "{}", priv_key_bech32).unwrap()
            }
            Key::ToPublic(args) => {
                let mut input = io::open_file_read(&args.input_key);
                let mut input_str = String::new();
                input
                    .read_to_string(&mut input_str)
                    .expect("Cannot read input key from the given input");

                let bech32: Bech32 = input_str
                    .trim_end()
                    .parse()
                    .expect("Expect a valid Bec32 string");

                let pub_key_bech32 = match bech32.hrp() {
                    Ed25519::SECRET_BECH32_HRP => gen_pub_key_bech32::<Ed25519>(bech32.data()),
                    Ed25519Bip32::SECRET_BECH32_HRP => {
                        gen_pub_key_bech32::<Ed25519Bip32>(bech32.data())
                    }
                    Ed25519Extended::SECRET_BECH32_HRP => {
                        gen_pub_key_bech32::<Ed25519Extended>(bech32.data())
                    }
                    FakeMMM::SECRET_BECH32_HRP => gen_pub_key_bech32::<FakeMMM>(bech32.data()),
                    Curve25519_2HashDH::SECRET_BECH32_HRP => {
                        gen_pub_key_bech32::<Curve25519_2HashDH>(bech32.data())
                    }
                    other => panic!("Unrecognized private key bech32 HRP: {}", other),
                };
                let mut file = io::open_file_write(&args.output);
                writeln!(file, "{}", pub_key_bech32).unwrap()
            }
        }
    }
}

fn gen_priv_key_bech32<K: AsymmetricKey>() -> Bech32 {
    let rng = ChaChaRng::from_rng(EntropyRng::new()).unwrap();
    let secret = K::generate(rng);
    let hrp = K::SECRET_BECH32_HRP.to_string();
    Bech32::new(hrp, secret.to_base32()).unwrap()
}

fn gen_pub_key_bech32<K: AsymmetricKey>(priv_key_bech32: &[u5]) -> Bech32 {
    let priv_key_bytes = Vec::<u8>::from_base32(priv_key_bech32).unwrap();
    let priv_key = K::secret_from_binary(&priv_key_bytes).unwrap();
    let pub_key = K::compute_public(&priv_key);
    let hrp = K::PUBLIC_BECH32_HRP.to_string();
    Bech32::new(hrp, pub_key.to_base32()).unwrap()
}

use bech32::{u5, Bech32, FromBase32, ToBase32};
use cardano::util::hex;
use chain_crypto::{
    AsymmetricKey, Curve25519_2HashDH, Ed25519, Ed25519Bip32, Ed25519Extended, FakeMMM,
};
use jcli_app::utils::io;
use rand::{rngs::EntropyRng, SeedableRng};
use rand_chacha::ChaChaRng;
use std::{
    io::Read as _,
    path::{Path, PathBuf},
};
use structopt::{clap::arg_enum, StructOpt};

custom_error! {pub Error
    Io { source: std::io::Error } = "I/O error",
    Bech32 { source: bech32::Error } = "Invalid Bech32",
    Hex { source: cardano::util::hex::Error } = "Invalid Hexadecimal",
    SecretKey { source: chain_crypto::SecretKeyError } = "Invalid secret key",
    Rand { source: rand::Error } = "Error while using random source",
    InvalidSeed { seed_len: usize } = "Invalid seed length, expected 32 bytes but received {seed_len}"
}

#[derive(StructOpt, Debug)]
#[structopt(name = "genesis", rename_all = "kebab-case")]
pub enum Key {
    /// generate a private key
    Generate(GenerateKeyArguments),
    /// get the public key out of a given private key
    ToPublic(ToPublicArguments),
    /// retrive a private key from the given bytes
    FromBytes(FromBytesArguments),
    /// get the bytes out of a private key
    ToBytes(ToBytesArguments),
}

#[derive(StructOpt, Debug)]
pub struct FromBytesArguments {
    /// Type of a private key
    ///
    /// value values are: ed25519, ed25510bip32, ed25519extended, curve25519_2hashdh
    #[structopt(long = "type")]
    pub key_type: GenPrivKeyType,

    /// retrieve the private key from the given bytes
    #[structopt(name = "INPUT_BYTES")]
    pub input_bytes: Option<PathBuf>,

    /// output the private key in the given file or to stdout if no
    /// value is provided.
    #[structopt(name = "OUTPUT_FILE")]
    pub output: Option<PathBuf>,
}

#[derive(StructOpt, Debug)]
pub struct ToBytesArguments {
    /// output to write bytes of the private key
    #[structopt(name = "OUTPUT_FILE")]
    pub output: Option<PathBuf>,

    /// path to the private key to serialize in bytes
    /// Or read from the standard input
    #[structopt(name = "INPUT_FILE")]
    pub input_key: Option<PathBuf>,
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

    /// optional seed to generate the key, for the same entropy the same key
    /// will be generated (32 bytes in hexadecimal). This seed will be fed to
    /// ChaChaRNG and allow pseudo random key generation. Do not use if you
    /// are not sure.
    #[structopt(
        long = "seed",
        short = "s",
        name = "SEED",
        parse(try_from_str = "hex::decode")
    )]
    pub seed: Option<Vec<u8>>,
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
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Key::Generate(args) => {
                let priv_key_bech32 = match args.key_type {
                    GenPrivKeyType::Ed25519 => gen_priv_key_bech32::<Ed25519>(args.seed)?,
                    GenPrivKeyType::Ed25519Bip32 => gen_priv_key_bech32::<Ed25519Bip32>(args.seed)?,
                    GenPrivKeyType::Ed25519Extended => {
                        gen_priv_key_bech32::<Ed25519Extended>(args.seed)?
                    }
                    GenPrivKeyType::FakeMMM => gen_priv_key_bech32::<FakeMMM>(args.seed)?,
                    GenPrivKeyType::Curve25519_2HashDH => {
                        gen_priv_key_bech32::<Curve25519_2HashDH>(args.seed)?
                    }
                };
                let mut file = io::open_file_write(&args.output);
                Ok(writeln!(file, "{}", priv_key_bech32)?)
            }
            Key::ToPublic(args) => {
                let bech32 = read_bech32(args.input_key)?;

                let pub_key_bech32 = match bech32.hrp() {
                    Ed25519::SECRET_BECH32_HRP => gen_pub_key_bech32::<Ed25519>(bech32.data())?,
                    Ed25519Bip32::SECRET_BECH32_HRP => {
                        gen_pub_key_bech32::<Ed25519Bip32>(bech32.data())?
                    }
                    Ed25519Extended::SECRET_BECH32_HRP => {
                        gen_pub_key_bech32::<Ed25519Extended>(bech32.data())?
                    }
                    FakeMMM::SECRET_BECH32_HRP => gen_pub_key_bech32::<FakeMMM>(bech32.data())?,
                    Curve25519_2HashDH::SECRET_BECH32_HRP => {
                        gen_pub_key_bech32::<Curve25519_2HashDH>(bech32.data())?
                    }
                    other => panic!("Unrecognized private key bech32 HRP: {}", other),
                };
                let mut file = io::open_file_write(&args.output);
                Ok(writeln!(file, "{}", pub_key_bech32)?)
            }
            Key::ToBytes(args) => {
                let bech32 = read_bech32(args.input_key)?;

                match bech32.hrp() {
                    Ed25519::PUBLIC_BECH32_HRP => {}
                    Ed25519Bip32::PUBLIC_BECH32_HRP => {}
                    Ed25519Extended::PUBLIC_BECH32_HRP => {}
                    FakeMMM::PUBLIC_BECH32_HRP => {}
                    Curve25519_2HashDH::PUBLIC_BECH32_HRP => {}
                    Ed25519::SECRET_BECH32_HRP => {}
                    Ed25519Bip32::SECRET_BECH32_HRP => {}
                    Ed25519Extended::SECRET_BECH32_HRP => {}
                    FakeMMM::SECRET_BECH32_HRP => {}
                    Curve25519_2HashDH::SECRET_BECH32_HRP => {}
                    other => panic!("Unrecognized private key bech32 HRP: {}", other),
                }
                use bech32::FromBase32;
                let bytes: Vec<u8> = Vec::from_base32(bech32.data())?;
                let mut file = io::open_file_write(&args.output);
                Ok(writeln!(file, "{}", cardano::util::hex::encode(&bytes))?)
            }
            Key::FromBytes(args) => {
                let bytes = read_hex(args.input_bytes)?;

                let priv_key_bech32 = match args.key_type {
                    GenPrivKeyType::Ed25519 => get_priv_key_from_bytes::<Ed25519>(&bytes)?,
                    GenPrivKeyType::Ed25519Bip32 => {
                        get_priv_key_from_bytes::<Ed25519Bip32>(&bytes)?
                    }
                    GenPrivKeyType::Ed25519Extended => {
                        get_priv_key_from_bytes::<Ed25519Extended>(&bytes)?
                    }
                    GenPrivKeyType::FakeMMM => get_priv_key_from_bytes::<FakeMMM>(&bytes)?,
                    GenPrivKeyType::Curve25519_2HashDH => {
                        get_priv_key_from_bytes::<Curve25519_2HashDH>(&bytes)?
                    }
                };
                let mut file = io::open_file_write(&args.output);
                Ok(writeln!(file, "{}", priv_key_bech32)?)
            }
        }
    }
}

fn read_hex<P: AsRef<Path>>(path: Option<P>) -> Result<Vec<u8>, Error> {
    let mut input = io::open_file_read(&path);
    let mut input_str = String::new();
    input.read_to_string(&mut input_str)?;
    Ok(cardano::util::hex::decode(&input_str.trim_end())?)
}

fn read_bech32<P: AsRef<Path>>(path: Option<P>) -> Result<Bech32, Error> {
    let mut input = io::open_file_read(&path);
    let mut input_str = String::new();
    input.read_to_string(&mut input_str)?;

    let bech32: Bech32 = input_str.trim_end().parse()?;
    Ok(bech32)
}

fn gen_priv_key_bech32<K: AsymmetricKey>(seed: Option<Vec<u8>>) -> Result<Bech32, Error> {
    let rng = if let Some(seed) = seed {
        if seed.len() != 32 {
            return Err(Error::InvalidSeed {
                seed_len: seed.len(),
            });
        }
        let mut seed_bytes = [0; 32];
        seed_bytes.copy_from_slice(&seed);
        ChaChaRng::from_seed(seed_bytes)
    } else {
        ChaChaRng::from_rng(EntropyRng::new())?
    };
    let secret = K::generate(rng);
    let hrp = K::SECRET_BECH32_HRP.to_string();
    Ok(Bech32::new(hrp, secret.to_base32())?)
}

fn gen_pub_key_bech32<K: AsymmetricKey>(priv_key_bech32: &[u5]) -> Result<Bech32, Error> {
    let priv_key_bytes = Vec::<u8>::from_base32(priv_key_bech32)?;
    let priv_key = K::secret_from_binary(&priv_key_bytes)?;
    let pub_key = K::compute_public(&priv_key);
    let hrp = K::PUBLIC_BECH32_HRP.to_string();
    Ok(Bech32::new(hrp, pub_key.to_base32())?)
}

fn get_priv_key_from_bytes<K: AsymmetricKey>(bytes: &[u8]) -> Result<String, Error> {
    use chain_crypto::bech32::Bech32 as _;
    let secret: chain_crypto::SecretKey<K> = chain_crypto::SecretKey::from_binary(bytes)?;
    Ok(secret.to_bech32_str())
}

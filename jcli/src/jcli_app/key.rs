use bech32::{u5, Bech32, FromBase32, ToBase32};
use cardano::util::hex;
use chain_crypto::{
    AsymmetricKey, AsymmetricPublicKey, Curve25519_2HashDH, Ed25519, Ed25519Bip32, Ed25519Extended,
    SumEd25519_12,
};
use jcli_app::utils::io;
use rand::{rngs::EntropyRng, SeedableRng};
use rand_chacha::ChaChaRng;
use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};
use structopt::{clap::arg_enum, StructOpt};

custom_error! { pub Error
    Io { source: std::io::Error } = "I/O error",
    Bech32 { source: bech32::Error } = "invalid Bech32",
    Hex { source: cardano::util::hex::Error } = "invalid Hexadecimal",
    SecretKey { source: chain_crypto::SecretKeyError } = "invalid secret key",
    Rand { source: rand::Error } = "error while using random source",
    InvalidSeed { seed_len: usize } = "invalid seed length, expected 32 bytes but received {seed_len}",
    InvalidInput { source: std::io::Error, path: PathBuf }
        = @{{ let _ = source; format_args!("invalid input file path '{}'", path.display()) }},
    InvalidOutput { source: std::io::Error, path: PathBuf }
        = @{{ let _ = source; format_args!("invalid output file path '{}'", path.display()) }},
    UnknownBech32PrivKeyHrp { hrp: String } = "unrecognized private key bech32 HRP: {hrp}",
}

#[derive(StructOpt, Debug)]
#[structopt(name = "genesis", rename_all = "kebab-case")]
pub enum Key {
    /// generate a private key
    Generate(Generate),
    /// get the public key out of a given private key
    ToPublic(ToPublic),
    /// retrive a private key from the given bytes
    FromBytes(FromBytes),
    /// get the bytes out of a private key
    ToBytes(ToBytes),
}

#[derive(StructOpt, Debug)]
pub struct FromBytes {
    /// Type of a private key
    ///
    /// value values are: ed25519, ed25510bip32, ed25519extended, curve25519_2hashdh or sumed25519_12
    #[structopt(long = "type")]
    key_type: GenPrivKeyType,

    /// retrieve the private key from the given bytes
    #[structopt(name = "INPUT_BYTES")]
    input_bytes: Option<PathBuf>,

    #[structopt(flatten)]
    output_file: OutputFile,
}

#[derive(StructOpt, Debug)]
pub struct ToBytes {
    #[structopt(flatten)]
    output_file: OutputFile,

    /// path to the private key to serialize in bytes
    /// Or read from the standard input
    #[structopt(name = "INPUT_FILE")]
    input_key: Option<PathBuf>,
}

#[derive(StructOpt, Debug)]
pub struct Generate {
    /// Type of a private key
    ///
    /// value values are: ed25519, ed25510bip32, ed25519extended, curve25519_2hashdh or sumed25519_12
    #[structopt(long = "type")]
    key_type: GenPrivKeyType,

    #[structopt(flatten)]
    output_file: OutputFile,

    /// optional seed to generate the key, for the same entropy the same key
    /// will be generated (32 bytes in hexadecimal). This seed will be fed to
    /// ChaChaRNG and allow pseudo random key generation. Do not use if you
    /// are not sure.
    #[structopt(long = "seed", short = "s", name = "SEED", parse(try_from_str))]
    seed: Option<Seed>,
}

#[derive(StructOpt, Debug)]
pub struct ToPublic {
    /// the source private key to extract the public key from
    ///
    /// if no value passed, the private key will be read from the
    /// standard input
    #[structopt(long = "input")]
    input_key: Option<PathBuf>,

    #[structopt(flatten)]
    output_file: OutputFile,
}

#[derive(StructOpt, Debug)]
struct OutputFile {
    /// output the key to the given file or to stdout if not provided
    #[structopt(name = "OUTPUT_FILE")]
    output: Option<PathBuf>,
}

impl OutputFile {
    fn open(&self) -> Result<impl Write, Error> {
        io::open_file_write(&self.output).map_err(|source| Error::InvalidOutput {
            source,
            path: self.output.clone().unwrap_or_default(),
        })
    }
}

arg_enum! {
    #[derive(StructOpt, Debug)]
    pub enum GenPrivKeyType {
        Ed25519,
        Ed25519Bip32,
        Ed25519Extended,
        SumEd25519_12,
        Curve25519_2HashDH,
    }
}

impl Key {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Key::Generate(args) => args.exec(),
            Key::ToPublic(args) => args.exec(),
            Key::ToBytes(args) => args.exec(),
            Key::FromBytes(args) => args.exec(),
        }
    }
}

impl Generate {
    fn exec(self) -> Result<(), Error> {
        let priv_key_bech32 = match self.key_type {
            GenPrivKeyType::Ed25519 => gen_priv_key::<Ed25519>(self.seed)?,
            GenPrivKeyType::Ed25519Bip32 => gen_priv_key::<Ed25519Bip32>(self.seed)?,
            GenPrivKeyType::Ed25519Extended => gen_priv_key::<Ed25519Extended>(self.seed)?,
            GenPrivKeyType::SumEd25519_12 => gen_priv_key::<SumEd25519_12>(self.seed)?,
            GenPrivKeyType::Curve25519_2HashDH => gen_priv_key::<Curve25519_2HashDH>(self.seed)?,
        };
        let mut output = self.output_file.open()?;
        writeln!(output, "{}", priv_key_bech32)?;
        Ok(())
    }
}

impl ToPublic {
    fn exec(self) -> Result<(), Error> {
        let bech32 = read_bech32(self.input_key)?;
        let data = bech32.data();
        let pub_key_bech32 = match bech32.hrp() {
            Ed25519::SECRET_BECH32_HRP => gen_pub_key::<Ed25519>(data),
            Ed25519Bip32::SECRET_BECH32_HRP => gen_pub_key::<Ed25519Bip32>(data),
            Ed25519Extended::SECRET_BECH32_HRP => gen_pub_key::<Ed25519Extended>(data),
            SumEd25519_12::SECRET_BECH32_HRP => gen_pub_key::<SumEd25519_12>(data),
            Curve25519_2HashDH::SECRET_BECH32_HRP => gen_pub_key::<Curve25519_2HashDH>(data),
            other => Err(Error::UnknownBech32PrivKeyHrp {
                hrp: other.to_string(),
            }),
        }?;
        let mut output = self.output_file.open()?;
        writeln!(output, "{}", pub_key_bech32)?;
        Ok(())
    }
}

impl ToBytes {
    fn exec(self) -> Result<(), Error> {
        let bech32 = read_bech32(self.input_key)?;

        match bech32.hrp() {
            Ed25519::PUBLIC_BECH32_HRP
            | Ed25519Bip32::PUBLIC_BECH32_HRP
            | SumEd25519_12::PUBLIC_BECH32_HRP
            | Curve25519_2HashDH::PUBLIC_BECH32_HRP
            | Ed25519::SECRET_BECH32_HRP
            | Ed25519Bip32::SECRET_BECH32_HRP
            | Ed25519Extended::SECRET_BECH32_HRP
            | SumEd25519_12::SECRET_BECH32_HRP
            | Curve25519_2HashDH::SECRET_BECH32_HRP => Ok(()),
            other => Err(Error::UnknownBech32PrivKeyHrp {
                hrp: other.to_string(),
            }),
        }?;
        let bytes = Vec::<u8>::from_base32(bech32.data())?;
        let mut output = self.output_file.open()?;
        writeln!(output, "{}", cardano::util::hex::encode(&bytes))?;
        Ok(())
    }
}

impl FromBytes {
    fn exec(self) -> Result<(), Error> {
        let bytes = read_hex(self.input_bytes)?;

        let priv_key_bech32 = match self.key_type {
            GenPrivKeyType::Ed25519 => bytes_to_priv_key::<Ed25519>(&bytes)?,
            GenPrivKeyType::Ed25519Bip32 => bytes_to_priv_key::<Ed25519Bip32>(&bytes)?,
            GenPrivKeyType::Ed25519Extended => bytes_to_priv_key::<Ed25519Extended>(&bytes)?,
            GenPrivKeyType::SumEd25519_12 => bytes_to_priv_key::<SumEd25519_12>(&bytes)?,
            GenPrivKeyType::Curve25519_2HashDH => bytes_to_priv_key::<Curve25519_2HashDH>(&bytes)?,
        };
        let mut output = self.output_file.open()?;
        writeln!(output, "{}", priv_key_bech32)?;
        Ok(())
    }
}

fn read_hex<P: AsRef<Path>>(path: Option<P>) -> Result<Vec<u8>, Error> {
    cardano::util::hex::decode(read_line(path)?.trim()).map_err(Into::into)
}

fn read_bech32<P: AsRef<Path>>(path: Option<P>) -> Result<Bech32, Error> {
    read_line(path)?.trim().parse().map_err(Into::into)
}

fn read_line<P: AsRef<Path>>(path: Option<P>) -> Result<String, Error> {
    let input = io::open_file_read(&path).map_err(|source| Error::InvalidInput {
        source,
        path: path
            .map(|path| path.as_ref().to_owned())
            .unwrap_or_default(),
    })?;
    let mut line = String::new();
    BufReader::new(input).read_line(&mut line)?;
    Ok(line)
}

fn gen_priv_key<K: AsymmetricKey>(seed: Option<Seed>) -> Result<Bech32, Error> {
    let rng = if let Some(seed) = seed {
        ChaChaRng::from_seed(seed.0)
    } else {
        ChaChaRng::from_rng(EntropyRng::new())?
    };
    let secret = K::generate(rng);
    let hrp = K::SECRET_BECH32_HRP.to_string();
    Ok(Bech32::new(hrp, secret.to_base32())?)
}

fn gen_pub_key<K: AsymmetricKey>(priv_key_bech32: &[u5]) -> Result<Bech32, Error> {
    let priv_key_bytes = Vec::<u8>::from_base32(priv_key_bech32)?;
    let priv_key = K::secret_from_binary(&priv_key_bytes)?;
    let pub_key = K::compute_public(&priv_key);
    let hrp = <K::PubAlg as AsymmetricPublicKey>::PUBLIC_BECH32_HRP.to_string();
    Ok(Bech32::new(hrp, pub_key.to_base32())?)
}

fn bytes_to_priv_key<K: AsymmetricKey>(bytes: &[u8]) -> Result<String, Error> {
    use chain_crypto::bech32::Bech32 as _;
    let secret: chain_crypto::SecretKey<K> = chain_crypto::SecretKey::from_binary(bytes)?;
    Ok(secret.to_bech32_str())
}

#[derive(Debug)]
struct Seed([u8; 32]);
impl std::str::FromStr for Seed {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let vec = hex::decode(s)?;
        if vec.len() != 32 {
            return Err(Error::InvalidSeed {
                seed_len: vec.len(),
            });
        }
        let mut bytes = [0; 32];
        bytes.copy_from_slice(&vec);
        Ok(Seed(bytes))
    }
}

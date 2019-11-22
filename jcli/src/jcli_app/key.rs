use crate::jcli_app::utils::io;
use bech32::{u5, Bech32, FromBase32, ToBase32};
use chain_crypto::{
    bech32::Bech32 as _, AsymmetricKey, AsymmetricPublicKey, Curve25519_2HashDH, Ed25519,
    Ed25519Bip32, Ed25519Extended, SecretKey, SigningAlgorithm, SumEd25519_12, Verification,
    VerificationAlgorithm,
};
use ed25519_bip32::{DerivationError, DerivationScheme};
use hex::FromHexError;
use rand::{rngs::EntropyRng, SeedableRng};
use rand_chacha::ChaChaRng;
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};
use structopt::{clap::arg_enum, StructOpt};

custom_error! { pub Error
    Io { source: std::io::Error } = "I/O error",
    Bech32 { source: bech32::Error } = "invalid Bech32",
    Hex { source: FromHexError } = "invalid Hexadecimal",
    SecretKey { source: chain_crypto::SecretKeyError } = "invalid secret key",
    PublicKey { source: chain_crypto::PublicKeyError } = "invalid public key",
    Signature { source: chain_crypto::SignatureError } = "invalid signature",
    Rand { source: rand::Error } = "error while using random source",
    InvalidSeed { seed_len: usize } = "invalid seed length, expected 32 bytes but received {seed_len}",
    InvalidInput { source: std::io::Error, path: PathBuf }
        = @{{ let _ = source; format_args!("invalid input file path '{}'", path.display()) }},
    InvalidOutput { source: std::io::Error, path: PathBuf }
        = @{{ let _ = source; format_args!("invalid output file path '{}'", path.display()) }},
    UnknownBech32PrivKeyHrp { hrp: String } = "unrecognized private key bech32 HRP: '{hrp}'",
    UnknownBech32PubKeyHrp { hrp: String } = "unrecognized public key bech32 HRP: '{hrp}'",
    UnexpectedBech32SignHrp { actual_hrp: String, expected_hrp: String }
        = "signature bech32 has invalid HRP: '{actual_hrp}', expected: '{expected_hrp}'",
    SignatureVerification = "signature verification failed",
    Derivation { source: DerivationError } = "failed to derive from BIP32 public key",
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
    /// sign data with private key
    Sign(Sign),
    /// verify signed data with public key
    Verify(Verify),
    /// derive a child key
    Derive(Derive),
}

#[derive(StructOpt, Debug)]
pub struct FromBytes {
    /// Type of a private key
    ///
    /// supported values are: ed25519, ed25519bip32, ed25519extended, curve25519_2hashdh or sumed25519_12
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
    /// supported values are: ed25519, ed25519bip32, ed25519extended, curve25519_2hashdh or sumed25519_12
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
pub struct Sign {
    /// path to file with bech32-encoded secret key
    ///
    /// supported key formats are: ed25519, ed25519bip32, ed25519extended and sumed25519_12
    #[structopt(long = "secret-key")]
    secret_key: PathBuf,

    /// path to file to write signature into, if no value is passed, standard output will be used
    #[structopt(long = "output", short = "o")]
    output: Option<PathBuf>,

    /// path to file with data to sign, if no value is passed, standard input will be used
    data: Option<PathBuf>,
}

#[derive(StructOpt, Debug)]
pub struct Verify {
    /// path to file with bech32-encoded public key
    ///
    /// supported key formats are: ed25519, ed25519bip32 and sumed25519_12
    #[structopt(long = "public-key")]
    public_key: PathBuf,

    /// path to file with signature
    #[structopt(long = "signature")]
    signature: PathBuf,

    /// path to file with signed data, if no value is passed, standard input will be used
    data: Option<PathBuf>,
}

#[derive(StructOpt, Debug)]
pub struct Derive {
    /// the parent key to derive a child key from
    ///
    /// if no value passed, the parent key will be read from the
    /// standard input
    #[structopt(long = "input")]
    parent_key: Option<PathBuf>,

    /// the index of child key
    index: u32,

    #[structopt(flatten)]
    child_key: OutputFile,
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
            Key::Sign(args) => args.exec(),
            Key::Verify(args) => args.exec(),
            Key::Derive(args) => args.exec(),
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
        let bech32 = read_bech32(&self.input_key)?;
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
        let bech32 = read_bech32(&self.input_key)?;

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
        writeln!(output, "{}", hex::encode(&bytes))?;
        Ok(())
    }
}

impl FromBytes {
    fn exec(self) -> Result<(), Error> {
        let bytes = read_hex(&self.input_bytes)?;

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

impl Sign {
    fn exec(self) -> Result<(), Error> {
        let secret_bech32 = read_bech32(&self.secret_key)?;
        let secret_bytes = Vec::<u8>::from_base32(secret_bech32.data())?;
        match secret_bech32.hrp() {
            Ed25519::SECRET_BECH32_HRP => self.sign::<Ed25519>(&secret_bytes),
            Ed25519Bip32::SECRET_BECH32_HRP => self.sign::<Ed25519Bip32>(&secret_bytes),
            Ed25519Extended::SECRET_BECH32_HRP => self.sign::<Ed25519Extended>(&secret_bytes),
            SumEd25519_12::SECRET_BECH32_HRP => self.sign::<SumEd25519_12>(&secret_bytes),
            other => Err(Error::UnknownBech32PrivKeyHrp {
                hrp: other.to_string(),
            }),
        }
    }

    fn sign<A>(self, secret_bytes: &[u8]) -> Result<(), Error>
    where
        A: SigningAlgorithm,
        <A as AsymmetricKey>::PubAlg: VerificationAlgorithm,
    {
        let secret = SecretKey::<A>::from_binary(secret_bytes)?;
        let mut data = Vec::new();
        io::open_file_read(&self.data)?.read_to_end(&mut data)?;
        let signature = secret.sign(&data);
        io::open_file_write(&self.output)?.write_all(signature.to_bech32_str().as_ref())?;
        Ok(())
    }
}

impl Verify {
    fn exec(self) -> Result<(), Error> {
        let public_bech32 = read_bech32(&self.public_key)?;
        let public_bytes = Vec::<u8>::from_base32(public_bech32.data())?;
        match public_bech32.hrp() {
            Ed25519::PUBLIC_BECH32_HRP => self.verify::<Ed25519>(&public_bytes),
            Ed25519Bip32::PUBLIC_BECH32_HRP => self.verify::<Ed25519Bip32>(&public_bytes),
            SumEd25519_12::PUBLIC_BECH32_HRP => self.verify::<SumEd25519_12>(&public_bytes),
            other => Err(Error::UnknownBech32PubKeyHrp {
                hrp: other.to_string(),
            }),
        }
    }

    fn verify<A>(self, public_bytes: &[u8]) -> Result<(), Error>
    where
        A: SigningAlgorithm,
        <A as AsymmetricKey>::PubAlg: VerificationAlgorithm,
    {
        let public = A::PubAlg::public_from_binary(&public_bytes)?;
        let sign_bech32 = read_bech32(&self.signature)?;
        if sign_bech32.hrp() != A::PubAlg::SIGNATURE_BECH32_HRP {
            return Err(Error::UnexpectedBech32SignHrp {
                actual_hrp: sign_bech32.hrp().to_string(),
                expected_hrp: A::PubAlg::SIGNATURE_BECH32_HRP.to_string(),
            });
        }
        let sign_bytes = Vec::<u8>::from_base32(sign_bech32.data())?;
        let sign = A::PubAlg::signature_from_bytes(&sign_bytes)?;
        let mut data = Vec::new();
        io::open_file_read(&self.data)?.read_to_end(&mut data)?;
        match A::PubAlg::verify_bytes(&public, &sign, &data) {
            Verification::Success => Ok(()),
            Verification::Failed => Err(Error::SignatureVerification),
        }?;
        println!("Success");
        Ok(())
    }
}

impl Derive {
    fn exec(self) -> Result<(), Error> {
        let key_bech32 = read_bech32(&self.parent_key)?;
        let key_bytes = Vec::<u8>::from_base32(key_bech32.data())?;
        let hrp;
        let child_key;

        match key_bech32.hrp() {
            Ed25519Bip32::PUBLIC_BECH32_HRP => {
                let key = Ed25519Bip32::public_from_binary(&key_bytes)?;
                child_key = key.derive(DerivationScheme::V2, self.index)?.to_base32();
                hrp = Ed25519Bip32::PUBLIC_BECH32_HRP.to_string();
            }
            Ed25519Bip32::SECRET_BECH32_HRP => {
                let key = Ed25519Bip32::secret_from_binary(&key_bytes)?;
                child_key = key.derive(DerivationScheme::V2, self.index).to_base32();
                hrp = Ed25519Bip32::SECRET_BECH32_HRP.to_string();
            }
            other => {
                return Err(Error::UnknownBech32PubKeyHrp {
                    hrp: other.to_string(),
                })
            }
        }

        let child_key_bech32 = Bech32::new(hrp, child_key)?;
        let mut output = self.child_key.open()?;
        writeln!(output, "{}", child_key_bech32)?;
        Ok(())
    }
}

fn read_hex<P: AsRef<Path>>(path: &Option<P>) -> Result<Vec<u8>, Error> {
    hex::decode(io::read_line(path)?).map_err(Into::into)
}

fn read_bech32<'a>(path: impl Into<Option<&'a PathBuf>>) -> Result<Bech32, Error> {
    io::read_line(&path.into())?.parse().map_err(Into::into)
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

use crate::jcli_lib::utils::io;
use crate::jcli_lib::utils::output_file::{self, OutputFile};
use bech32::{self, u5, FromBase32, ToBase32};
use chain_crypto::{
    bech32::Bech32 as _, AsymmetricKey, AsymmetricPublicKey, Curve25519_2HashDh, Ed25519,
    Ed25519Bip32, Ed25519Extended, SecretKey, SigningAlgorithm, SumEd25519_12, Verification,
    VerificationAlgorithm,
};
use ed25519_bip32::{DerivationError, DerivationScheme};
use hex::FromHexError;
use rand::{rngs::OsRng, SeedableRng};
use rand_chacha::ChaChaRng;
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};
use structopt::{clap::arg_enum, StructOpt};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error("invalid Bech32")]
    Bech32(#[from] bech32::Error),
    #[error("invalid Hexadecimal")]
    Hex(#[from] FromHexError),
    #[error("invalid secret key")]
    SecretKey(#[from] chain_crypto::SecretKeyError),
    #[error("invalid public key")]
    PublicKey(#[from] chain_crypto::PublicKeyError),
    #[error("invalid signature")]
    Signature(#[from] chain_crypto::SignatureError),
    #[error("error while using random source")]
    Rand(#[from] rand::Error),
    #[error("invalid seed length, expected 32 bytes but received {seed_len}")]
    InvalidSeed { seed_len: usize },
    #[error(transparent)]
    InvalidOutput(#[from] output_file::Error),
    #[error("unrecognized private key bech32 HRP: '{hrp}'")]
    UnknownBech32PrivKeyHrp { hrp: String },
    #[error("unrecognized public key bech32 HRP: '{hrp}'")]
    UnknownBech32PubKeyHrp { hrp: String },
    #[error("signature bech32 has invalid HRP: '{actual_hrp}', expected: '{expected_hrp}'")]
    UnexpectedBech32SignHrp {
        actual_hrp: String,
        expected_hrp: String,
    },
    #[error("signature verification failed")]
    SignatureVerification,
    #[error("failed to derive from BIP32 public key")]
    Derivation(#[from] DerivationError),
    #[error("ed25519bip32 key expected, signature bech32 has invalid HRP: '{actual_hrp}', expected: '{public_hrp}' or '{private_hrp}'")]
    UnexpectedBip32Bech32Hrp {
        actual_hrp: String,
        public_hrp: String,
        private_hrp: String,
    },
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
    /// derive a child key from a ed25519bip32 parent key
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
    /// the ed25519bip32 parent key to derive a child key from
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

arg_enum! {
    #[derive(StructOpt, Debug)]
    pub enum GenPrivKeyType {
        Ed25519,
        Ed25519Bip32,
        Ed25519Extended,
        SumEd25519_12,
        Curve25519_2HashDh,
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
            GenPrivKeyType::Curve25519_2HashDh => gen_priv_key::<Curve25519_2HashDh>(self.seed)?,
        };
        let mut output = self.output_file.open()?;
        writeln!(output, "{}", priv_key_bech32)?;
        Ok(())
    }
}

impl ToPublic {
    fn exec(self) -> Result<(), Error> {
        let (hrp, data) = read_bech32(&self.input_key)?;
        let pub_key_bech32 = match hrp.as_ref() {
            Ed25519::SECRET_BECH32_HRP => gen_pub_key::<Ed25519>(&data),
            Ed25519Bip32::SECRET_BECH32_HRP => gen_pub_key::<Ed25519Bip32>(&data),
            Ed25519Extended::SECRET_BECH32_HRP => gen_pub_key::<Ed25519Extended>(&data),
            SumEd25519_12::SECRET_BECH32_HRP => gen_pub_key::<SumEd25519_12>(&data),
            Curve25519_2HashDh::SECRET_BECH32_HRP => gen_pub_key::<Curve25519_2HashDh>(&data),
            _ => Err(Error::UnknownBech32PrivKeyHrp { hrp }),
        }?;
        let mut output = self.output_file.open()?;
        writeln!(output, "{}", pub_key_bech32)?;
        Ok(())
    }
}

impl ToBytes {
    fn exec(self) -> Result<(), Error> {
        let (hrp, data) = read_bech32(&self.input_key)?;

        match hrp.as_ref() {
            Ed25519::PUBLIC_BECH32_HRP
            | Ed25519Bip32::PUBLIC_BECH32_HRP
            | SumEd25519_12::PUBLIC_BECH32_HRP
            | Curve25519_2HashDh::PUBLIC_BECH32_HRP
            | Ed25519::SECRET_BECH32_HRP
            | Ed25519Bip32::SECRET_BECH32_HRP
            | Ed25519Extended::SECRET_BECH32_HRP
            | SumEd25519_12::SECRET_BECH32_HRP
            | Curve25519_2HashDh::SECRET_BECH32_HRP => Ok(()),
            _ => Err(Error::UnknownBech32PrivKeyHrp { hrp }),
        }?;
        let bytes = Vec::<u8>::from_base32(&data)?;
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
            GenPrivKeyType::Curve25519_2HashDh => bytes_to_priv_key::<Curve25519_2HashDh>(&bytes)?,
        };
        let mut output = self.output_file.open()?;
        writeln!(output, "{}", priv_key_bech32)?;
        Ok(())
    }
}

impl Sign {
    fn exec(self) -> Result<(), Error> {
        let (hrp, data) = read_bech32(&self.secret_key)?;
        let secret_bytes = Vec::<u8>::from_base32(&data)?;
        match hrp.as_ref() {
            Ed25519::SECRET_BECH32_HRP => self.sign::<Ed25519>(&secret_bytes),
            Ed25519Bip32::SECRET_BECH32_HRP => self.sign::<Ed25519Bip32>(&secret_bytes),
            Ed25519Extended::SECRET_BECH32_HRP => self.sign::<Ed25519Extended>(&secret_bytes),
            SumEd25519_12::SECRET_BECH32_HRP => self.sign::<SumEd25519_12>(&secret_bytes),
            _ => Err(Error::UnknownBech32PrivKeyHrp { hrp }),
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
        let (hrp, data) = read_bech32(&self.public_key)?;
        let public_bytes = Vec::<u8>::from_base32(&data)?;
        match hrp.as_ref() {
            Ed25519::PUBLIC_BECH32_HRP => self.verify::<Ed25519>(&public_bytes),
            Ed25519Bip32::PUBLIC_BECH32_HRP => self.verify::<Ed25519Bip32>(&public_bytes),
            SumEd25519_12::PUBLIC_BECH32_HRP => self.verify::<SumEd25519_12>(&public_bytes),
            _ => Err(Error::UnknownBech32PubKeyHrp { hrp }),
        }
    }

    fn verify<A>(self, public_bytes: &[u8]) -> Result<(), Error>
    where
        A: SigningAlgorithm,
        <A as AsymmetricKey>::PubAlg: VerificationAlgorithm,
    {
        let public = A::PubAlg::public_from_binary(&public_bytes)?;
        let (hrp, data) = read_bech32(&self.signature)?;
        if hrp != A::PubAlg::SIGNATURE_BECH32_HRP {
            return Err(Error::UnexpectedBech32SignHrp {
                actual_hrp: hrp,
                expected_hrp: A::PubAlg::SIGNATURE_BECH32_HRP.to_string(),
            });
        }
        let sign_bytes = Vec::<u8>::from_base32(&data)?;
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
        let (phrp, pdata) = read_bech32(&self.parent_key)?;
        let key_bytes = Vec::<u8>::from_base32(&pdata)?;
        let hrp;
        let child_key;

        match phrp.as_ref() {
            Ed25519Bip32::PUBLIC_BECH32_HRP => {
                let key = Ed25519Bip32::public_from_binary(&key_bytes)?;
                child_key = key.derive(DerivationScheme::V2, self.index)?.to_base32();
                hrp = Ed25519Bip32::PUBLIC_BECH32_HRP;
            }
            Ed25519Bip32::SECRET_BECH32_HRP => {
                let key = Ed25519Bip32::secret_from_binary(&key_bytes)?;
                child_key = key.derive(DerivationScheme::V2, self.index).to_base32();
                hrp = Ed25519Bip32::SECRET_BECH32_HRP;
            }
            other => {
                return Err(Error::UnexpectedBip32Bech32Hrp {
                    actual_hrp: other.to_string(),
                    public_hrp: Ed25519Bip32::PUBLIC_BECH32_HRP.to_string(),
                    private_hrp: Ed25519Bip32::SECRET_BECH32_HRP.to_string(),
                })
            }
        }

        let child_key_bech32 = bech32::encode(&hrp, child_key)?;
        let mut output = self.child_key.open()?;
        writeln!(output, "{}", child_key_bech32)?;
        Ok(())
    }
}

fn read_hex<P: AsRef<Path>>(path: &Option<P>) -> Result<Vec<u8>, Error> {
    hex::decode(io::read_line(path)?).map_err(Into::into)
}

fn read_bech32<'a>(
    path: impl Into<Option<&'a PathBuf>>,
) -> Result<(String, Vec<bech32::u5>), Error> {
    let line = io::read_line(&path.into())?;
    bech32::decode(&line).map_err(Into::into)
}

fn gen_priv_key<K: AsymmetricKey>(seed: Option<Seed>) -> Result<String, Error> {
    let rng = if let Some(seed) = seed {
        ChaChaRng::from_seed(seed.0)
    } else {
        ChaChaRng::from_rng(OsRng)?
    };
    let secret = K::generate(rng);
    let hrp = K::SECRET_BECH32_HRP;
    Ok(bech32::encode(hrp, secret.to_base32())?)
}

fn gen_pub_key<K: AsymmetricKey>(priv_key_bech32: &[u5]) -> Result<String, Error> {
    let priv_key_bytes = Vec::<u8>::from_base32(priv_key_bech32)?;
    let priv_key = K::secret_from_binary(&priv_key_bytes)?;
    let pub_key = K::compute_public(&priv_key);
    let hrp = <K::PubAlg as AsymmetricPublicKey>::PUBLIC_BECH32_HRP;
    Ok(bech32::encode(hrp, pub_key.to_base32())?)
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

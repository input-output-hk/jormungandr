use crate::jcli_app::vote::{Error, OutputFile, Seed};
use chain_vote::MemberCommunicationKey;
use rand::rngs::OsRng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Generate {
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
#[structopt(rename_all = "kebab-case")]
pub enum CommunicationKey {
    /// generate a private key
    Generate(Generate),
    /// get the public key out of a given private key
    ToPublic(ToPublic),
}

impl Generate {
    fn exec(self) -> Result<(), Error> {
        let mut rng = if let Some(seed) = self.seed {
            ChaCha20Rng::from_seed(seed.0)
        } else {
            ChaCha20Rng::from_rng(OsRng)?
        };

        let key = MemberCommunicationKey::new(&mut rng);

        let mut output = self.output_file.open()?;
        writeln!(output, "{}", hex::encode(key.to_bytes()))?;
        Ok(())
    }
}

impl ToPublic {
    fn exec(self) -> Result<(), Error> {
        let bytes = read_hex(&self.input_key)?;

        let key =
            chain_vote::gargamel::SecretKey::from_bytes(&bytes).ok_or(Error::InvalidSecretKey)?;

        let kp = chain_vote::gargamel::Keypair::from_secretkey(key);

        let mut output = self.output_file.open()?;
        writeln!(output, "{}", hex::encode(kp.public_key.to_bytes()))?;

        Ok(())
    }
}

impl CommunicationKey {
    pub fn exec(self) -> Result<(), super::Error> {
        match self {
            CommunicationKey::Generate(args) => args.exec(),
            CommunicationKey::ToPublic(args) => args.exec(),
        }
    }
}

fn read_hex<P: AsRef<Path>>(path: &Option<P>) -> Result<Vec<u8>, Error> {
    hex::decode(crate::jcli_app::utils::io::read_line(path)?).map_err(Into::into)
}

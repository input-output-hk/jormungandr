use crate::jcli_lib::vote::{Error, OutputFile, Seed};
use chain_crypto::bech32::Bech32;
use chain_vote::MemberCommunicationKey;
use rand::rngs::OsRng;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use std::{io::Write, path::PathBuf};
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
    /// The file with the private key to extract the public key from.
    /// If no value passed, the private key will be read from the
    /// standard input.
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
        writeln!(output, "{}", key.to_bech32_str())?;
        Ok(())
    }
}

impl ToPublic {
    fn exec(self) -> Result<(), Error> {
        let line = crate::jcli_lib::utils::io::read_line(&self.input_key)?;

        let sk = MemberCommunicationKey::try_from_bech32_str(&line)?.to_public();

        let mut output = self.output_file.open()?;
        writeln!(output, "{}", sk.to_bech32_str())?;

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

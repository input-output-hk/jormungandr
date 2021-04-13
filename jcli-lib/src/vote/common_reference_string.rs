use super::{Error, OutputFile, Seed};
use rand::rngs::OsRng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::io::Write;
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
#[structopt(rename_all = "kebab-case")]
pub enum Crs {
    /// generate the common reference string
    Generate(Generate),
}

impl Generate {
    fn exec(self) -> Result<(), Error> {
        let mut rng = if let Some(seed) = self.seed {
            ChaCha20Rng::from_seed(seed.0)
        } else {
            ChaCha20Rng::from_rng(OsRng)?
        };

        let crs = chain_vote::CRS::random(&mut rng);

        let mut output = self.output_file.open()?;
        writeln!(output, "{}", hex::encode(crs.to_bytes().as_ref()))?;

        Ok(())
    }
}

impl Crs {
    pub fn exec(self) -> Result<(), super::Error> {
        match self {
            Crs::Generate(args) => args.exec(),
        }
    }
}

use crate::jcli_lib::vote::{Error, OutputFile, Seed};
use chain_crypto::bech32::Bech32;
use chain_vote::committee::{
    MemberCommunicationPublicKey, MemberPublicKey, MemberSecretKey, MemberState,
};
use rand::rngs::OsRng;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use std::{convert::TryInto, io::Write, path::PathBuf};
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Generate {
    /// threshold number of the committee members sufficient for
    /// decrypting the tally
    #[structopt(long, short, name = "THRESHOLD", parse(try_from_str))]
    threshold: usize,

    /// the common reference string
    #[structopt(long, name = "Crs")]
    crs: String,

    /// communication keys of all committee members
    #[structopt(long, short, name = "COMMUNICATION_KEYS",
        parse(try_from_str = MemberCommunicationPublicKey::try_from_bech32_str),
        required = true,
    )]
    keys: Vec<MemberCommunicationPublicKey>,

    /// index of the committee member this key is generated for
    #[structopt(long, short, name = "INDEX", parse(try_from_str))]
    index: u64,

    /// optional seed to generate the key, for the same entropy the same key
    /// will be generated (32 bytes in hexadecimal). This seed will be fed to
    /// ChaChaRNG and allow pseudo random key generation. Do not use if you
    /// are not sure.
    #[structopt(long = "seed", short = "s", name = "SEED", parse(try_from_str))]
    seed: Option<Seed>,

    #[structopt(flatten)]
    output_file: OutputFile,
}

#[derive(StructOpt)]
pub struct ToPublic {
    /// The file with the private key to extract the public key from.
    /// If no value passed, the private key will be read from the
    /// standard input.
    #[structopt(long = "input")]
    input_key: Option<PathBuf>,

    #[structopt(flatten)]
    output_file: OutputFile,
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum MemberKey {
    /// generate a private key
    Generate(Generate),
    /// get the public key out of a given private key
    ToPublic(ToPublic),
}

impl Generate {
    fn exec(self) -> Result<(), Error> {
        let mut rng = match self.seed {
            Some(seed) => ChaCha20Rng::from_seed(seed.0),
            None => ChaCha20Rng::from_rng(OsRng)?,
        };

        // this things are asserted in MemberState::new, but it's better to not
        // panic here
        let n = self.keys.len();
        if self.threshold == 0 || self.threshold > n {
            return Err(Error::InvalidThreshold {
                threshold: self.threshold,
                committee_members: n,
            });
        }
        if self.index as usize >= n {
            return Err(Error::InvalidCommitteMemberIndex);
        }

        let crs = chain_vote::Crs::from_hash(self.crs.as_bytes());

        let ms = MemberState::new(
            &mut rng,
            self.threshold,
            &crs,
            &self.keys,
            self.index.try_into().unwrap(),
        );

        let key = ms.secret_key();

        let mut output = self.output_file.open()?;
        writeln!(output, "{}", key.to_bech32_str())?;
        Ok(())
    }
}

impl ToPublic {
    fn exec(self) -> Result<(), Error> {
        let line = crate::jcli_lib::utils::io::read_line(&self.input_key)?;

        let pk: MemberPublicKey = MemberSecretKey::try_from_bech32_str(&line)?.to_public();

        let mut output = self.output_file.open()?;
        writeln!(output, "{}", pk.to_bech32_str())?;

        Ok(())
    }
}

impl MemberKey {
    pub fn exec(self) -> Result<(), super::Error> {
        match self {
            MemberKey::Generate(args) => args.exec(),
            MemberKey::ToPublic(args) => args.exec(),
        }
    }
}

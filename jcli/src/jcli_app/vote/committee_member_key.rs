use super::{Error, OutputFile, Seed};
use chain_vote::gargamel::PublicKey;
use chain_vote::{MemberCommunicationPublicKey, MemberState};
use rand::rngs::OsRng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::{
    convert::TryInto,
    io::Write,
    path::{Path, PathBuf},
};
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Generate {
    /// optional seed to generate the key, for the same entropy the same key
    /// will be generated (32 bytes in hexadecimal). This seed will be fed to
    /// ChaChaRNG and allow pseudo random key generation. Do not use if you
    /// are not sure.
    #[structopt(long = "seed", short = "s", name = "SEED", parse(try_from_str))]
    seed: Option<Seed>,

    /// the committee member index (my)
    index: u64,

    /// the common reference string
    #[structopt(parse(try_from_str = parse_crs))]
    crs: chain_vote::CRS,

    threshold: usize,

    #[structopt(
        long = "communication_keys",
        short = "c",
        name = "COMMUNICATION_KEYS",
        parse(try_from_str = parse_member_communication_key)
    )]
    communication_keys: Vec<MemberCommunicationPublicKey>,

    #[structopt(flatten)]
    output_file: OutputFile,
}

fn parse_member_communication_key(key: &str) -> Result<MemberCommunicationPublicKey, Error> {
    let raw_key = hex::decode(key)?;
    let pk = PublicKey::from_bytes(&raw_key).ok_or(Error::InvalidPublicKey)?;
    Ok(MemberCommunicationPublicKey::from_public_key(pk))
}

fn parse_crs(crs: &str) -> Result<chain_vote::CRS, Error> {
    let bytes = hex::decode(crs)?;

    chain_vote::CRS::from_bytes(&bytes).ok_or(Error::InvalidCrs)
}

#[derive(StructOpt)]
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

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum CommitteeMemberKey {
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

        if self.communication_keys.is_empty() {
            return Err(Error::EmptyCommittee);
        }

        // this things are asserted in MemberState::new, but it's better to not
        // panic here
        let n = self.communication_keys.len();
        if self.threshold == 0 || self.threshold > n {
            return Err(Error::InvalidThreshold {
                threshold: self.threshold,
                committee_members: n,
            });
        }
        if self.index as usize >= n {
            return Err(Error::InvalidCommitteMemberIndex);
        }

        let ms = MemberState::new(
            &mut rng,
            self.threshold,
            &self.crs,
            &self.communication_keys,
            self.index.try_into().unwrap(),
        );

        let key = ms.secret_key();

        let mut output = self.output_file.open()?;
        writeln!(output, "{}", hex::encode(key.to_bytes()))?;
        Ok(())
    }
}

impl ToPublic {
    fn exec(self) -> Result<(), Error> {
        let key = read_hex(&self.input_key)?;

        let key =
            chain_vote::gargamel::SecretKey::from_bytes(&key).ok_or(Error::InvalidSecretKey)?;

        let pk = chain_vote::gargamel::Keypair::from_secretkey(key).public_key;

        let mut output = self.output_file.open()?;
        writeln!(output, "{}", hex::encode(pk.to_bytes()))?;

        Ok(())
    }
}

impl CommitteeMemberKey {
    pub fn exec(self) -> Result<(), super::Error> {
        match self {
            CommitteeMemberKey::Generate(args) => args.exec(),
            CommitteeMemberKey::ToPublic(args) => args.exec(),
        }
    }
}

fn read_hex<P: AsRef<Path>>(path: &Option<P>) -> Result<Vec<u8>, Error> {
    hex::decode(crate::jcli_app::utils::io::read_line(path)?).map_err(Into::into)
}

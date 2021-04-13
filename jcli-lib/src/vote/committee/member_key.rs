use crate::vote::{Error, OutputFile, Seed};
use bech32::{FromBase32, ToBase32};
use chain_vote::gargamel::PublicKey;
use chain_vote::{MemberCommunicationPublicKey, MemberState};
use rand::rngs::OsRng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::{convert::TryInto, io::Write, path::PathBuf};
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Generate {
    /// threshold number of the committee members sufficient for
    /// decrypting the tally
    #[structopt(long, short, name = "THRESHOLD", parse(try_from_str))]
    threshold: usize,

    /// the common reference string
    #[structopt(long, name = "CRS", parse(try_from_str = parse_crs))]
    crs: chain_vote::CRS,

    /// communication keys of all committee members
    #[structopt(long, short, name = "COMMUNICATION_KEYS",
        parse(try_from_str = parse_member_communication_key),
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

        let ms = MemberState::new(
            &mut rng,
            self.threshold,
            &self.crs,
            &self.keys,
            self.index.try_into().unwrap(),
        );

        let key = ms.secret_key();

        let mut output = self.output_file.open()?;
        writeln!(
            output,
            "{}",
            bech32::encode(
                crate::vote::bech32_constants::MEMBER_SK_HRP,
                key.to_bytes().to_base32()
            )
            .map_err(Error::Bech32)?
        )?;
        Ok(())
    }
}

impl ToPublic {
    fn exec(self) -> Result<(), Error> {
        let line = crate::utils::io::read_line(&self.input_key)?;
        let (hrp, key) = bech32::decode(&line).map_err(Error::Bech32)?;

        if hrp != crate::vote::bech32_constants::MEMBER_SK_HRP {
            return Err(Error::InvalidSecretKey);
        }

        let key = chain_vote::gargamel::SecretKey::from_bytes(
            &Vec::<u8>::from_base32(&key).map_err(|_| Error::InvalidSecretKey)?,
        )
        .ok_or(Error::InvalidSecretKey)?;

        let pk = chain_vote::gargamel::Keypair::from_secretkey(key).public_key;

        let mut output = self.output_file.open()?;
        let key = bech32::encode(
            crate::vote::bech32_constants::MEMBER_PK_HRP,
            pk.to_bytes().to_base32(),
        )
        .map_err(Error::Bech32)?;
        writeln!(output, "{}", key)?;

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

fn parse_member_communication_key(key: &str) -> Result<MemberCommunicationPublicKey, Error> {
    let (hrp, raw_key) = bech32::decode(key).map_err(Error::Bech32)?;

    if hrp != crate::vote::bech32_constants::COMMUNICATION_PK_HRP {
        return Err(Error::InvalidPublicKey);
    }

    let pk = PublicKey::from_bytes(&Vec::<u8>::from_base32(&raw_key).map_err(Error::Bech32)?)
        .ok_or(Error::InvalidPublicKey)?;
    Ok(MemberCommunicationPublicKey::from_public_key(pk))
}

fn parse_crs(crs: &str) -> Result<chain_vote::CRS, Error> {
    let bytes = hex::decode(crs)?;

    chain_vote::CRS::from_bytes(&bytes).ok_or(Error::InvalidCrs)
}

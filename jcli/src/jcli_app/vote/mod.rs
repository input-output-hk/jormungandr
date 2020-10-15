mod committee_communication_key;
mod committee_member_key;
mod common_reference_string;
mod encrypting_vote_key;

use structopt::StructOpt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error("invalid Hexadecimal")]
    Hex(#[from] hex::FromHexError),
    #[error("error while using random source")]
    Rand(#[from] rand::Error),
    #[error("invalid seed length, expected 32 bytes but received {seed_len}")]
    InvalidSeed { seed_len: usize },
    #[error("invalid output file path '{path}'")]
    InvalidOutput {
        #[source]
        source: std::io::Error,
        path: std::path::PathBuf,
    },
    #[error("invalid public key")]
    InvalidPublicKey,
    #[error("invalid secret key")]
    InvalidSecretKey,
    #[error("invalid common reference string")]
    InvalidCrs,
    #[error("must provide at least one member key to build encrypting vote key")]
    EncryptingVoteKeyFromEmpty,
    #[error("expected at least one committee key")]
    EmptyCommittee,
    #[error("threshold should be in range (0..{committee_members:?}] and is {threshold:?}")]
    InvalidThreshold {
        threshold: usize,
        committee_members: usize,
    },
    #[error("invalid committee member index")]
    InvalidCommitteMemberIndex,
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Vote {
    /// Build a commitee communication key
    CommitteeCommunicationKey(committee_communication_key::CommitteeCommunicationKey),
    /// Build a committee member key
    CommitteeMemberKey(committee_member_key::CommitteeMemberKey),
    /// Build an encryption vote key
    EncryptingVoteKey(encrypting_vote_key::EncryptingVoteKey),
    /// Build an encryption vote key
    CRS(common_reference_string::CRS),
}

impl Vote {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Vote::CommitteeCommunicationKey(cmd) => cmd.exec(),
            Vote::CommitteeMemberKey(cmd) => cmd.exec(),
            Vote::EncryptingVoteKey(cmd) => cmd.exec(),
            Vote::CRS(cmd) => cmd.exec(),
        }
    }
}

// FIXME: Duplicated with key.rs
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

// FIXME: Duplicated with key.rs
#[derive(StructOpt, Debug)]
struct OutputFile {
    /// output the key to the given file or to stdout if not provided
    #[structopt(name = "OUTPUT_FILE")]
    output: Option<std::path::PathBuf>,
}

impl OutputFile {
    fn open(&self) -> Result<impl std::io::Write, Error> {
        crate::jcli_app::utils::io::open_file_write(&self.output).map_err(|source| {
            Error::InvalidOutput {
                source,
                path: self.output.clone().unwrap_or_default(),
            }
        })
    }
}

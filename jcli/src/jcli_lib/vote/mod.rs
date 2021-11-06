use crate::jcli_lib::utils::output_file::{self, OutputFile};
use crate::jcli_lib::utils::{
    key_parser,
    vote::{SharesError, VotePlanError},
};
use crate::rest;
use crate::utils::io;
use chain_core::property::Serialize;
use chain_impl_mockchain::fragment::Fragment;
use std::io::Write;
use std::path::PathBuf;
use structopt::StructOpt;
use thiserror::Error;

mod committee;
mod election_public_key;
mod tally;
mod update_proposal;
mod update_vote;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error("invalid Hexadecimal")]
    Hex(#[from] hex::FromHexError),
    #[error("base64 decode error")]
    Base64(#[from] base64::DecodeError),
    #[error("bech32 error")]
    Bech32(#[from] chain_crypto::bech32::Error),
    #[error("error while decoding base64 source")]
    Rand(#[from] rand::Error),
    #[error("invalid seed length, expected 32 bytes but received {seed_len}")]
    InvalidSeed { seed_len: usize },
    #[error(transparent)]
    InvalidOutput(#[from] output_file::Error),
    #[error("invalid public key")]
    InvalidPublicKey,
    #[error("invalid secret key")]
    InvalidSecretKey,
    #[error("invalid common reference string")]
    InvalidCrs,
    #[error("threshold should be in range (0..{committee_members:?}] and is {threshold:?}")]
    InvalidThreshold {
        threshold: usize,
        committee_members: usize,
    },
    #[error("invalid committee member index")]
    InvalidCommitteMemberIndex,
    #[error("failed to read encrypted tally bytes")]
    EncryptedTallyRead,
    #[error("failed to read decryption key bytes")]
    DecryptionKeyRead,
    #[error("expected encrypted private tally, found {found}")]
    PrivateTallyExpected { found: &'static str },
    #[error(transparent)]
    TallyError(#[from] chain_vote::tally::TallyError),
    #[error(transparent)]
    FormatError(#[from] crate::jcli_lib::utils::output_format::Error),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error(transparent)]
    VotePlanError(#[from] VotePlanError),
    #[error(transparent)]
    SharesError(#[from] SharesError),
    #[error("could not process secret file '{0}'")]
    SecretKeyReadFailed(#[from] key_parser::Error),
    #[error(transparent)]
    RestError(#[from] rest::Error),
    #[error("invalid input file path '{path}'")]
    InputInvalid {
        #[source]
        source: std::io::Error,
        path: PathBuf,
    },
    #[error("config file corrupted")]
    ConfigFileCorrupted(#[source] serde_yaml::Error),
    #[error("could not open fragment file '{path}'")]
    FragmentFileOpenFailed {
        #[source]
        source: std::io::Error,
        path: PathBuf,
    },
    #[error("could not write fragment file '{path}'")]
    FragmentFileWriteFailed {
        #[source]
        source: std::io::Error,
        path: PathBuf,
    },
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Vote {
    /// Create committee member keys
    Committee(committee::Committee),
    /// Build the election public key from committee member keys
    ElectionKey(election_public_key::ElectionPublicKey),
    /// Perform decryption of private voting tally
    Tally(tally::Tally),
    /// Create proposal for updating chain config
    UpdateProposal(update_proposal::UpdateProposal),
    /// Vote for the update proposal
    UpdateVote(update_vote::UpdateVote),
}

impl Vote {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Vote::Committee(cmd) => cmd.exec(),
            Vote::ElectionKey(cmd) => cmd.exec(),
            Vote::Tally(cmd) => cmd.exec(),
            Vote::UpdateProposal(cmd) => cmd.exec(),
            Vote::UpdateVote(cmd) => cmd.exec(),
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

fn write_fragment_into_file(
    fragment: Fragment,
    fragment_file: Option<PathBuf>,
) -> Result<(), Error> {
    let fragment_bytes = fragment.serialize_as_vec()?;

    let hex = hex::encode(&fragment_bytes);

    let mut writer =
        io::open_file_write(&fragment_file).map_err(|source| Error::FragmentFileOpenFailed {
            source,
            path: io::path_to_path_buf(&fragment_file),
        })?;

    writer
        .write_all(hex.as_bytes())
        .map_err(|source| Error::FragmentFileWriteFailed {
            source,
            path: io::path_to_path_buf(&fragment_file),
        })?;

    Ok(())
}

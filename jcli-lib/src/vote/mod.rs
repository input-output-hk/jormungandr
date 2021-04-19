//! Voting operations.
use crate::utils::output_file::{self, OutputFile};
use crate::utils::vote::{SharesError, VotePlanError};
use crate::key::Seed;

pub mod bech32_constants;
mod committee;
mod common_reference_string;
mod encrypting_vote_key;
mod tally;

#[cfg(feature = "structopt")]
use structopt::StructOpt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error("invalid Hexadecimal")]
    Hex(#[from] hex::FromHexError),
    #[error("base64 decode error")]
    Base64(#[from] base64::DecodeError),
    #[error("bech32 decode error")]
    Bech32(#[from] bech32::Error),
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
    TallyError(#[from] chain_vote::TallyError),
    #[error(transparent)]
    FormatError(#[from] crate::utils::output_format::Error),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error(transparent)]
    VotePlanError(#[from] VotePlanError),
    #[error(transparent)]
    SharesError(#[from] SharesError),
}

#[cfg_attr(
    feature = "structopt",
    derive(StructOpt),
    structopt(rename_all = "kebab-case")
)]
pub enum Vote {
    /// Create committee member keys
    Committee(committee::Committee),
    /// Build an encryption key from committee member keys
    EncryptingKey(encrypting_vote_key::EncryptingVoteKey),
    /// Create a common reference string
    Crs(common_reference_string::Crs),
    /// Perform decryption of private voting tally
    Tally(tally::Tally),
}

impl Vote {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Vote::Committee(cmd) => cmd.exec(),
            Vote::EncryptingKey(cmd) => cmd.exec(),
            Vote::Crs(cmd) => cmd.exec(),
            Vote::Tally(cmd) => cmd.exec(),
        }
    }
}

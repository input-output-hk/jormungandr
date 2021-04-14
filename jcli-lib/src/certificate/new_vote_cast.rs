use crate::certificate::{write_cert, Error};
use crate::utils;
use bech32::FromBase32;
use chain_impl_mockchain::{
    certificate::{Certificate, VoteCast, VotePlanId},
    vote::{Choice, Payload},
};
use rand_chacha::rand_core::SeedableRng;
use std::path::PathBuf;
#[cfg(feature = "structopt")]
use structopt::StructOpt;

#[cfg_attr(feature = "structopt", derive(StructOpt))]
pub struct PublicVoteCast {
    /// the vote plan identified on the blockchain
    #[cfg_attr(feature = "structopt", structopt(long = "vote-plan-id"))]
    vote_plan_id: VotePlanId,

    /// the number of proposal in the vote plan you vote for
    #[cfg_attr(feature = "structopt", structopt(long = "proposal-index"))]
    proposal_index: u8,

    /// the number of choice within the proposal you vote for
    #[cfg_attr(feature = "structopt", structopt(long = "choice"))]
    choice: u8,

    /// write the output to the given file or print it to the standard output if not defined
    #[cfg_attr(feature = "structopt", structopt(long = "output"))]
    output: Option<PathBuf>,
}

#[cfg_attr(feature = "structopt", derive(StructOpt))]
pub struct PrivateVoteCast {
    /// the vote plan identified on the blockchain
    #[cfg_attr(feature = "structopt", structopt(long = "vote-plan-id"))]
    vote_plan_id: VotePlanId,

    /// the number of proposal in the vote plan you vote for
    #[cfg_attr(feature = "structopt", structopt(long = "proposal-index"))]
    proposal_index: u8,

    /// size of voting options
    #[cfg_attr(feature = "structopt", structopt(long = "options-size"))]
    options: usize,

    /// the number of choice within the proposal you vote for
    #[cfg_attr(feature = "structopt", structopt(long = "choice"))]
    choice: u8,

    /// key to encrypt the vote with
    #[cfg_attr(feature = "structopt", structopt(long = "key-path"))]
    encrypting_key_path: Option<PathBuf>,

    /// write the output to the given file or print it to the standard output if not defined
    #[cfg_attr(feature = "structopt", structopt(long = "output"))]
    output: Option<PathBuf>,
}

/// create a vote cast certificate
#[cfg_attr(feature = "structopt", derive(StructOpt))]
pub enum VoteCastCmd {
    Public(PublicVoteCast),
    Private(PrivateVoteCast),
}

impl PublicVoteCast {
    pub fn exec(self) -> Result<(), Error> {
        let payload = Payload::Public {
            choice: Choice::new(self.choice),
        };

        let vote_cast = VoteCast::new(self.vote_plan_id, self.proposal_index, payload);
        let cert = Certificate::VoteCast(vote_cast);
        write_cert(self.output.as_deref(), cert.into())
    }
}

impl PrivateVoteCast {
    pub fn exec(self) -> Result<(), Error> {
        let mut rng = rand_chacha::ChaChaRng::from_entropy();
        let key_line = utils::io::read_line(&self.encrypting_key_path)?;
        let (hrp, data) = bech32::decode(&key_line).map_err(Error::InvalidBech32)?;
        if hrp != crate::vote::bech32_constants::ENCRYPTING_VOTE_PK_HRP {
            return Err(Error::InvalidBech32Key {
                expected: crate::vote::bech32_constants::ENCRYPTING_VOTE_PK_HRP.to_string(),
                actual: hrp,
            });
        }
        let key_bin = Vec::<u8>::from_base32(&data)?;
        let key =
            chain_vote::EncryptingVoteKey::from_bytes(&key_bin).ok_or(Error::VoteEncryptingKey)?;

        let vote = chain_vote::Vote::new(self.options, self.choice as usize);
        let (encrypted_vote, proof) =
            chain_impl_mockchain::vote::encrypt_vote(&mut rng, &key, vote);

        let payload = Payload::Private {
            encrypted_vote,
            proof,
        };

        let vote_cast = VoteCast::new(self.vote_plan_id, self.proposal_index, payload);
        let cert = Certificate::VoteCast(vote_cast);
        write_cert(self.output.as_deref(), cert.into())
    }
}

impl VoteCastCmd {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            VoteCastCmd::Public(vote_cast) => vote_cast.exec(),
            VoteCastCmd::Private(vote_cast) => vote_cast.exec(),
        }
    }
}

use crate::jcli_lib::certificate::{write_cert, Error};
use crate::jcli_lib::utils;
use chain_crypto::bech32::Bech32;
use chain_impl_mockchain::{
    certificate::{Certificate, VoteCast, VotePlanId},
    vote::{Choice, Payload},
};
use rand_chacha::rand_core::SeedableRng;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct PublicVoteCast {
    /// the vote plan identified on the blockchain
    #[structopt(long = "vote-plan-id")]
    vote_plan_id: VotePlanId,

    /// the number of proposal in the vote plan you vote for
    #[structopt(long = "proposal-index")]
    proposal_index: u8,

    /// the number of choice within the proposal you vote for
    #[structopt(long = "choice")]
    choice: u8,

    /// write the output to the given file or print it to the standard output if not defined
    #[structopt(long = "output")]
    output: Option<PathBuf>,
}

#[derive(StructOpt)]
pub struct PrivateVoteCast {
    /// the vote plan identified on the blockchain
    #[structopt(long = "vote-plan-id")]
    vote_plan_id: VotePlanId,

    /// the number of proposal in the vote plan you vote for
    #[structopt(long = "proposal-index")]
    proposal_index: u8,

    /// size of voting options
    #[structopt(long = "options-size")]
    options: usize,

    /// the number of choice within the proposal you vote for
    #[structopt(long = "choice")]
    choice: u8,

    /// key to encrypt the vote with
    #[structopt(long = "key-path")]
    election_key_path: Option<PathBuf>,

    /// write the output to the given file or print it to the standard output if not defined
    #[structopt(long = "output")]
    output: Option<PathBuf>,
}

/// create a vote cast certificate
#[derive(StructOpt)]
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
        let key_line = utils::io::read_line(&self.election_key_path)?;
        let key = chain_vote::ElectionPublicKey::try_from_bech32_str(&key_line)?;

        let vote = chain_vote::Vote::new(self.options, self.choice as usize);
        let crs = chain_vote::Crs::from_hash(self.vote_plan_id.as_ref());
        let (encrypted_vote, proof) =
            chain_impl_mockchain::vote::encrypt_vote(&mut rng, &crs, &key, vote);

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

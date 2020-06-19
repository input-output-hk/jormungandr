use crate::jcli_app::certificate::{write_cert, Error};
use chain_impl_mockchain::{
    certificate::{Certificate, VoteCast, VotePlanId},
    vote::{Choice, Payload},
};
use std::path::PathBuf;
use structopt::StructOpt;

/// create a vote cast certificate
#[derive(StructOpt)]
pub struct VoteCastCmd {
    /// the vote plan identified on the blockchain
    pub vote_plan_id: VotePlanId,

    /// the number of proposal in the vote plan you vote for
    pub proposal_index: u8,

    /// the number of choice within the proposal you vote for
    pub choice: u8,

    /// should be used for vote plans with public votes
    #[structopt(long = "public")]
    pub public: bool,

    /// write the output to the given file or print it to the standard output if not defined
    #[structopt(long = "output")]
    pub output: Option<PathBuf>,
}

impl VoteCastCmd {
    pub fn exec(self) -> Result<(), Error> {
        let payload = if self.public {
            Payload::Public {
                choice: Choice::new(self.choice),
            }
        } else {
            unimplemented!("private votes are not supported yet");
        };

        let vote_cast = VoteCast::new(self.vote_plan_id, self.proposal_index, payload);
        let cert = Certificate::VoteCast(vote_cast);
        write_cert(self.output.as_deref(), cert.into())
    }
}

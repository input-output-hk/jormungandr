mod cast;
mod tally;
mod tx;

use cast::CastVote;
use tally::VoteTally;
use tx::SendTransaction;

use crate::controller::UserInteractionController;
use crate::test::Result;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Send {
    /// Sends transaction
    Tx(SendTransaction),
    /// Tally the vote
    Tally(VoteTally),
    /// Send the vote
    Vote(CastVote),
}

impl Send {
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<()> {
        match self {
            Send::Tx(transaction) => transaction.exec(controller),
            Send::Tally(vote_tally) => vote_tally.exec(controller),
            Send::Vote(cast_vote) => cast_vote.exec(controller),
        }
    }
}

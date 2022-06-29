mod cast;
mod tally;
mod tx;

use crate::controller::{Error, UserInteractionController};
use cast::CastVote;
use structopt::StructOpt;
use tally::VoteTally;
use tx::SendTransaction;

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
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<(), Error> {
        match self {
            Send::Tx(transaction) => transaction.exec(controller),
            Send::Tally(vote_tally) => vote_tally.exec(controller),
            Send::Vote(cast_vote) => cast_vote.exec(controller),
        }
    }
}

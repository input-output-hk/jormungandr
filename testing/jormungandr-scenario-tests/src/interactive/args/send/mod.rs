mod tally;
mod tx;

use tally::VoteTally;
use tx::SendTransaction;

use super::UserInteractionController;
use crate::test::Result;
use structopt::StructOpt;


#[derive(StructOpt, Debug)]
pub enum Send {
    /// Sends transaction
    Tx(SendTransaction),
    /// Tally the vote
    Tally(VoteTally),

}

impl Send {
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<()> {
        match self {
            Send::Tx(transaction) => transaction.exec(controller),
            Send::Tally(vote_tally) => vote_tally.exec(controller),
        }
    }
}

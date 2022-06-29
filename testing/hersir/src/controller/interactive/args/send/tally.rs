use super::UserInteractionController;
use crate::{controller::Error, style};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct VoteTally {
    #[structopt(short = "c", long = "committee")]
    pub committee: String,
    #[structopt(short = "p", long = "vote-plan")]
    pub vote_plan: String,
    #[structopt(short = "v", long = "via")]
    pub via: String,
}

impl VoteTally {
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<(), Error> {
        let mem_pool_check = controller.tally_vote(&self.committee, &self.vote_plan, &self.via)?;
        println!(
            "{}",
            style::info.apply_to(format!(
                "tally vote fragment '{}' successfully sent",
                mem_pool_check.fragment_id()
            ))
        );
        Ok(())
    }
}

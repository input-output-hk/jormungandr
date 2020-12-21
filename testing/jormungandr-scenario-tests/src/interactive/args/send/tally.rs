use super::UserInteractionController;
use crate::{style, test::Result};
use jortestkit::prelude::InteractiveCommandError;
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
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<()> {
        let node = controller
            .nodes()
            .iter()
            .cloned()
            .find(|x| *x.alias() == self.via);
        let legacy_node = controller
            .legacy_nodes()
            .iter()
            .cloned()
            .find(|x| *x.alias() == self.via);

        if let Some(node) = node {
            let mem_pool_check = controller.tally_vote(&self.committee, &self.vote_plan, &node)?;
            println!(
                "{}",
                style::info.apply_to(format!(
                    "tally vote fragment '{}' successfully sent",
                    mem_pool_check.fragment_id()
                ))
            );
            return Ok(());
        } else if let Some(legacy_node) = legacy_node {
            let mem_pool_check =
                controller.tally_vote(&self.committee, &self.vote_plan, &legacy_node)?;
            println!(
                "{}",
                style::info.apply_to(format!(
                    "tally vote fragment '{}' successfully sent",
                    mem_pool_check.fragment_id()
                ))
            );
            return Ok(());
        }

        Err(
            InteractiveCommandError::UserError(format!("alias not found {}", self.via.clone()))
                .into(),
        )
    }
}

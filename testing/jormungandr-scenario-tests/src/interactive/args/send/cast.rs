use super::UserInteractionController;
use crate::{style, test::Result};
use jortestkit::prelude::InteractiveCommandError;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct CastVote {
    #[structopt(short = "w", long = "wallet")]
    pub wallet: String,
    #[structopt(short = "p", long = "vote-plan")]
    pub vote_plan: String,
    #[structopt(short = "v", long = "via")]
    pub via: String,

    #[structopt(short = "i", long = "idx")]
    pub proposal_index: Option<usize>,

    #[structopt(short = "d", long = "id")]
    pub proposal_id: Option<String>,

    #[structopt(short = "c", long = "choice")]
    pub choice: u8,
}

impl CastVote {
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<()> {
        let proposal_index = self.proposal_index.unwrap_or_else(|| {
            let vote_plan = controller
                .controller()
                .vote_plan(&self.vote_plan)
                .expect("cannot find vote plan");
            if let Some(id) = &self.proposal_id {
                let (index, _) = vote_plan
                    .proposals()
                    .iter()
                    .enumerate()
                    .find(|(_idx, x)| hex::encode(x.id()) == *id)
                    .expect("cannot find proposal");
                return index;
            }
            panic!("Either proposal_index or proposal id has to be provided")
        });

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
            let mem_pool_check = controller.cast_vote(
                &self.wallet,
                &self.vote_plan,
                &node,
                proposal_index,
                self.choice,
            )?;
            println!(
                "{}",
                style::info.apply_to(format!(
                    "vote cast fragment '{}' successfully sent",
                    mem_pool_check.fragment_id()
                ))
            );
            return Ok(());
        } else if let Some(legacy_node) = legacy_node {
            let mem_pool_check = controller.cast_vote(
                &self.wallet,
                &self.vote_plan,
                &legacy_node,
                proposal_index,
                self.choice,
            )?;
            println!(
                "{}",
                style::info.apply_to(format!(
                    "vote cast fragment '{}' successfully sent",
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

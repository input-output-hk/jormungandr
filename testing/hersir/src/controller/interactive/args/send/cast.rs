use super::UserInteractionController;
use crate::{controller::Error, style};
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
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<(), Error> {
        let proposal_index = self.proposal_index.unwrap_or_else(|| {
            let vote_plan = controller
                .controller()
                .defined_vote_plan(&self.vote_plan)
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

        let mem_pool_check = controller.cast_vote(
            &self.wallet,
            &self.vote_plan,
            &self.via,
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
        Ok(())
    }
}

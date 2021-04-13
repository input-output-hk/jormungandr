mod stake_pool_id;
mod vote_plan_id;

use crate::jcli_lib::certificate::Error;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum ShowArgs {
    /// get the stake pool id from the given stake pool registration certificate
    StakePoolId(stake_pool_id::GetStakePoolId),
    /// get the vote plan id from the given vote plan certificate
    VotePlanId(vote_plan_id::GetVotePlanId),
}

impl ShowArgs {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            ShowArgs::StakePoolId(args) => args.exec(),
            ShowArgs::VotePlanId(args) => args.exec(),
        }
    }
}

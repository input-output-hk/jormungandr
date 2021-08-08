use crate::jcli_lib::rest::{Error, RestArgs};
use crate::jcli_lib::utils::OutputFormat;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct AccountVotes {
    #[structopt(flatten)]
    args: RestArgs,

    #[structopt(flatten)]
    output_format: OutputFormat,

    /// Account address to filter votes
    #[structopt(short, long)]
    account: String,

    /// Id of the voteplan for which we want to list proposals
    /// the account voted for
    #[structopt(short, long)]
    voteplan_id: String,
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Vote {
    /// Get numbers of proposals within a vote plan a given user has voted for
    AccountVotes(AccountVotes),
}

impl Vote {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Vote::AccountVotes(cmd) => cmd.exec(),
        }
    }
}

impl AccountVotes {
    fn exec(self) -> Result<(), Error> {
        let response = self
            .args
            .client()?
            .get(&[
                "v1",
                "votes",
                "plan",
                &self.voteplan_id,
                "account-votes",
                &self.account,
            ])
            .execute()?
            .json()?;
        let formatted = self.output_format.format_json(response)?;
        println!("{}", formatted);
        Ok(())
    }
}

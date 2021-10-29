use crate::jcli_lib::rest::{Error, RestArgs};
use crate::jcli_lib::utils::OutputFormat;
use crate::utils::AccountId;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct AccountVotes {
    #[structopt(flatten)]
    args: RestArgs,

    #[structopt(flatten)]
    output_format: OutputFormat,

    /// Account id to filter votes.
    /// An Account ID either in the form of an address of kind account, or an account public key.
    #[structopt(short, long, parse(try_from_str = AccountId::try_from_str))]
    account_id: AccountId,

    /// Id of the voteplan for which we want to list proposals
    /// the account voted for
    #[structopt(short, long)]
    vote_plan_id: String,
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
                &self.vote_plan_id,
                "account-votes",
                &self.account_id.to_url_arg(),
            ])
            .execute()?
            .json()?;
        let formatted = self.output_format.format_json(response)?;
        println!("{}", formatted);
        Ok(())
    }
}

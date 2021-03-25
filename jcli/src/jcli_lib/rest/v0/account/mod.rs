use crate::jcli_lib::rest::{Error, RestArgs};
use crate::jcli_lib::utils::{AccountId, OutputFormat};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Account {
    /// Get account state
    Get {
        #[structopt(flatten)]
        args: RestArgs,
        #[structopt(flatten)]
        output_format: OutputFormat,
        /// An Account ID either in the form of an address of kind account, or an account public key
        #[structopt(parse(try_from_str = AccountId::try_from_str))]
        account_id: AccountId,
    },
}

impl Account {
    pub fn exec(self) -> Result<(), Error> {
        let Account::Get {
            args,
            output_format,
            account_id,
        } = self;
        let state = args
            .client()?
            .get(&["v0", "account", &account_id.to_url_arg()])
            .execute()?
            .json()?;
        let formatted = output_format.format_json(state)?;
        println!("{}", formatted);
        Ok(())
    }
}

use crate::jcli_app::rest::{Error, RestArgs};
use crate::jcli_app::utils::{AccountId, OutputFormat};
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
        let state = args.request_json_with_args(
            &["v0", "account", &account_id.to_url_arg()],
            |client, url| client.get(url),
        )?;
        let formatted = output_format.format_json(state)?;
        println!("{}", formatted);
        Ok(())
    }
}

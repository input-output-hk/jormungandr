use jcli_app::rest::Error;
use jcli_app::utils::{AccountId, DebugFlag, HostAddr, OutputFormat, RestApiSender};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Account {
    /// Get account state
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
        #[structopt(flatten)]
        output_format: OutputFormat,
        /// An Account ID either in the form of an address of kind account, or an account public key
        #[structopt(parse(try_from_str = "AccountId::try_from_str"))]
        account_id: AccountId,
    },
}

impl Account {
    pub fn exec(self) -> Result<(), Error> {
        let Account::Get {
            addr,
            debug,
            output_format,
            account_id,
        } = self;
        let url = addr
            .with_segments(&["v0", "account", &account_id.to_url_arg()])?
            .into_url();
        let builder = reqwest::Client::new().get(url);
        let response = RestApiSender::new(builder, &debug).send()?;
        response.ok_response()?;
        let state = response.body().json_value()?;
        let formatted = output_format.format_json(state)?;
        println!("{}", formatted);
        Ok(())
    }
}

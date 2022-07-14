use crate::jcli_lib::{
    rest::{Error, RestArgs},
    utils::OutputFormat,
};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Leaders {
    /// Leadership log operations
    Logs(GetLogs),
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum GetLogs {
    /// Get leadership log
    Get {
        #[structopt(flatten)]
        args: RestArgs,
        #[structopt(flatten)]
        output_format: OutputFormat,
    },
}

impl Leaders {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Leaders::Logs(GetLogs::Get {
                args,
                output_format,
            }) => get_logs(args, output_format),
        }
    }
}

fn get_logs(args: RestArgs, output_format: OutputFormat) -> Result<(), Error> {
    let response = args
        .client()?
        .get(&["v0", "leaders", "logs"])
        .execute()?
        .json()?;
    let formatted = output_format.format_json(response)?;
    println!("{}", formatted);
    Ok(())
}

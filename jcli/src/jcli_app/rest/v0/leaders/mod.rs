use jcli_app::rest::Error;
use jcli_app::utils::{io, DebugFlag, HostAddr, OutputFormat, RestApiSender};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Leaders {
    /// Get list of leader IDs
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
        #[structopt(flatten)]
        output_format: OutputFormat,
    },
    /// Register new leader and get its ID
    Post {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
        /// File containing YAML with leader secret.
        /// It must have the same format as secret YAML passed to Jormungandr as --secret.
        /// If not provided, YAML will be read from stdin.
        #[structopt(short, long)]
        file: Option<PathBuf>,
    },
    /// Delete leader
    Delete {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
        /// ID of deleted leader
        id: u32,
    },

    /// Leadership log operations
    Logs(GetLogs),
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum GetLogs {
    /// Get leadership log
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
        #[structopt(flatten)]
        output_format: OutputFormat,
    },
}

impl Leaders {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Leaders::Get {
                addr,
                debug,
                output_format,
            } => get(addr, debug, output_format),
            Leaders::Post { addr, debug, file } => post(addr, debug, file),
            Leaders::Delete { id, addr, debug } => delete(addr, debug, id),
            Leaders::Logs(GetLogs::Get {
                addr,
                debug,
                output_format,
            }) => get_logs(addr, debug, output_format),
        }
    }
}

fn get(addr: HostAddr, debug: DebugFlag, output_format: OutputFormat) -> Result<(), Error> {
    let url = addr.with_segments(&["v0", "leaders"])?.into_url();
    let builder = reqwest::Client::new().get(url);
    let response = RestApiSender::new(builder, &debug).send()?;
    response.ok_response()?;
    let leaders = response.body().json_value()?;
    let formatted = output_format.format_json(leaders)?;
    println!("{}", formatted);
    Ok(())
}

fn post(addr: HostAddr, debug: DebugFlag, file: Option<PathBuf>) -> Result<(), Error> {
    let url = addr.with_segments(&["v0", "leaders"])?.into_url();
    let builder = reqwest::Client::new().post(url);
    let input: serde_json::Value = io::read_yaml(&file)?;
    let response = RestApiSender::new(builder, &debug)
        .with_json_body(&input)?
        .send()?;
    response.ok_response()?;
    println!("{}", response.body().text().as_ref());
    Ok(())
}

fn delete(addr: HostAddr, debug: DebugFlag, id: u32) -> Result<(), Error> {
    let url = addr
        .with_segments(&["v0", "leaders", &id.to_string()])?
        .into_url();
    let builder = reqwest::Client::new().delete(url);
    let response = RestApiSender::new(builder, &debug).send()?;
    response.ok_response()?;
    println!("Success");
    Ok(())
}

fn get_logs(addr: HostAddr, debug: DebugFlag, output_format: OutputFormat) -> Result<(), Error> {
    let url = addr.with_segments(&["v0", "leaders", "logs"])?.into_url();
    let builder = reqwest::Client::new().get(url);
    let response = RestApiSender::new(builder, &debug).send()?;
    response.ok_response()?;
    let logs = response.body().json_value()?;
    let formatted = output_format.format_json(logs)?;
    println!("{}", formatted);
    Ok(())
}

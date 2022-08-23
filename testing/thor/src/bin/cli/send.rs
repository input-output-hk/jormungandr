use crate::cli::command::Error;
use chain_addr::AddressReadable;
use jcli_lib::utils::io::open_file_read;
use jormungandr_lib::interfaces::VotePlan;
use std::path::PathBuf;
use structopt::StructOpt;
use thor::cli::CliController;

#[derive(StructOpt, Debug)]
pub struct SendCommand {
    // pin
    #[structopt(long, short)]
    pub wait: bool,

    #[structopt(subcommand)] // Note that we mark a field as a subcommand
    cmd: SendSubCommand,
}

impl SendCommand {
    pub fn exec(self, controller: CliController) -> Result<(), Error> {
        match self.cmd {
            SendSubCommand::Tx(send_tx) => send_tx.exec(controller, self.wait),
            SendSubCommand::VotePlan(send_vote_plan) => send_vote_plan.exec(controller, self.wait),
        }
    }
}

#[derive(StructOpt, Debug)]
pub enum SendSubCommand {
    Tx(TxCommand),
    VotePlan(SendVotePlanCommand),
}

#[derive(StructOpt, Debug)]
pub struct TxCommand {
    /// address in bech32 format
    #[structopt(long)]
    pub address: AddressReadable,

    /// ada to send
    #[structopt(long)]
    pub ada: u64,

    // pin
    #[structopt(long, short)]
    pub pin: String,
}

impl TxCommand {
    pub fn exec(self, mut controller: CliController, wait: bool) -> Result<(), Error> {
        controller.transaction(&self.pin, wait, self.address.to_address().into(), self.ada)?;
        controller.save_config().map_err(Into::into)
    }
}

#[derive(StructOpt, Debug)]
pub struct SendVotePlanCommand {
    /// vote plan json
    #[structopt(long)]
    pub vote_plan: PathBuf,

    // pin
    #[structopt(long, short)]
    pub pin: String,
}

impl SendVotePlanCommand {
    pub fn exec(self, mut controller: CliController, wait: bool) -> Result<(), Error> {
        let configuration = open_file_read(&Some(self.vote_plan))?;
        let vpc: VotePlan = serde_yaml::from_reader(configuration)?;
        controller.send_vote_plan(&self.pin, wait, vpc)?;
        controller.save_config().map_err(Into::into)
    }
}

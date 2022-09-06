use crate::cli::command::Error;
use chain_addr::AddressReadable;
use jcli_lib::utils::io::open_file_read;
use jormungandr_lib::interfaces::VotePlan;
use std::path::PathBuf;
use structopt::StructOpt;
use thor::cli::{Alias, CliController};

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
            SendSubCommand::Vote(send_vote) => send_vote.exec(controller, self.wait),
        }
    }
}

#[derive(StructOpt, Debug)]
pub enum SendSubCommand {
    /// Send transaction
    Tx(TxCommand),
    /// Send vote related transactions
    Vote(SendVoteCommand),
}

#[derive(StructOpt, Debug)]
pub enum SendVoteCommand {
    VotePlan(SendVotePlanCommand),
    Tally(SendTallyVote),
}

impl SendVoteCommand {
    pub fn exec(self, controller: CliController, wait: bool) -> Result<(), Error> {
        match self {
            Self::VotePlan(vote_plan) => vote_plan.exec(controller, wait),
            Self::Tally(send_tally_vote) => send_tally_vote.exec(controller, wait),
        }
    }
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

#[derive(StructOpt, Debug)]
pub enum SendTallyVote {
    Public(SendPublicTallyVotePlanCommand),
    Private(SendPrivateTallyVotePlanCommand),
}

impl SendTallyVote {
    pub fn exec(self, controller: CliController, wait: bool) -> Result<(), Error> {
        match self {
            Self::Public(public) => public.exec(controller, wait),
            Self::Private(private) => private.exec(controller, wait),
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct SendPublicTallyVotePlanCommand {
    /// vote plan json
    #[structopt(long)]
    pub vote_plan_id: String,

    // pin
    #[structopt(long, short)]
    pub pin: String,
}

impl SendPublicTallyVotePlanCommand {
    pub fn exec(self, mut controller: CliController, wait: bool) -> Result<(), Error> {
        controller.send_public_vote_tally(&self.pin, wait, self.vote_plan_id)?;
        controller.save_config().map_err(Into::into)
    }
}

#[derive(StructOpt, Debug)]
pub struct SendPrivateTallyVotePlanCommand {
    /// vote plan json
    #[structopt(long)]
    pub vote_plan_id: String,

    /// member key alias
    #[structopt(long)]
    pub member_key_alias: Alias,

    /// pin
    #[structopt(long, short)]
    pub pin: String,
}

impl SendPrivateTallyVotePlanCommand {
    pub fn exec(self, mut controller: CliController, wait: bool) -> Result<(), Error> {
        controller.send_private_vote_tally(
            &self.pin,
            wait,
            self.vote_plan_id,
            self.member_key_alias,
        )?;
        controller.save_config().map_err(Into::into)
    }
}

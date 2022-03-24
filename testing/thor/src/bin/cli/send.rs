use crate::cli::command::Error;
use chain_addr::AddressReadable;
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
    pub fn exec(self, contoller: CliController) -> Result<(), Error> {
        match self.cmd {
            SendSubCommand::Tx(send_tx) => send_tx.exec(contoller, self.wait),
        }
    }
}

#[derive(StructOpt, Debug)]
pub enum SendSubCommand {
    Tx(Tx),
}

#[derive(StructOpt, Debug)]
pub struct Tx {
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

impl Tx {
    pub fn exec(self, mut contoller: CliController, wait: bool) -> Result<(), Error> {
        contoller.transaction(&self.pin, wait, self.address.to_address().into(), self.ada)?;
        contoller.save_config().map_err(Into::into)
    }
}

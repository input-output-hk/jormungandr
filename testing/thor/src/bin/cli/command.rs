use super::wallet::Wallets;
use crate::cli::send::SendCommand;
use structopt::StructOpt;
use thiserror::Error;
use thor::cli::{CliController, Connection};

#[derive(StructOpt, Debug)]
pub enum Command {
    /// connect to backend
    Connect(Connect),
    /// get Address
    Address,
    /// Prints wallet status
    Status,
    /// clear transaction
    ClearTx,
    /// confirms transaction
    ConfirmTx,
    /// Prints wallets, nodes which can be used. Draw topology
    Refresh,
    Logs,
    Statuses,
    Send(SendCommand),
    PendingTransactions,
    Wallets(Wallets),
}

const DELIMITER: &str = "===================";

fn print_delim() {
    println!("{}", DELIMITER);
}

impl Command {
    pub fn exec(self, mut controller: CliController) -> Result<(), Error> {
        match self {
            Command::Wallets(wallets) => wallets.exec(controller),
            Command::Connect(connect) => connect.exec(controller),
            Command::Address => {
                let wallet = controller.wallets().wallet()?;
                println!("Address: {}", wallet.address_readable()?);
                println!("Account id: {}", wallet.id()?);
                Ok(())
            }
            Command::Status => {
                let account_state = controller.account_state()?;
                print_delim();
                println!("- Delegation: {:?}", account_state.delegation());
                println!("- Value: {}", account_state.value());
                println!("- Spending counters: {:?}", account_state.counters());
                println!("- Rewards: {:?}", account_state.last_rewards());
                println!("- Tokens: {:?}", account_state.tokens());
                print_delim();
                Ok(())
            }
            Command::PendingTransactions => {
                print_delim();
                for (idx, fragment_ids) in
                    controller.wallets().wallet()?.pending_tx.iter().enumerate()
                {
                    println!("{}. {}", (idx + 1), fragment_ids);
                }
                print_delim();
                Ok(())
            }
            Command::ConfirmTx => {
                controller.confirm_tx()?;
                controller.save_config().map_err(Into::into)
            }
            Command::ClearTx => {
                controller.clear_txs()?;
                controller.save_config().map_err(Into::into)
            }
            Command::Refresh => {
                controller.refresh_state()?;
                controller.save_config().map_err(Into::into)
            }
            Command::Logs => {
                println!("{:#?}", controller.fragment_logs()?);
                Ok(())
            }
            Command::Statuses => {
                print_delim();
                for (idx, (id, status)) in controller.statuses()?.iter().enumerate() {
                    println!("{}. {} -> {:#?}", idx, id, status);
                }
                print_delim();
                Ok(())
            }
            Command::Send(send) => send.exec(controller),
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct Connect {
    #[structopt(name = "ADDRESS")]
    pub address: String,

    /// uses https for sending fragments
    #[structopt(short = "s", long = "https")]
    pub use_https: bool,

    /// uses https for sending fragments
    #[structopt(short = "d", long = "enable-debug")]
    pub enable_debug: bool,
}

impl Connect {
    pub fn exec(&self, mut controller: CliController) -> Result<(), Error> {
        controller.update_connection(Connection {
            address: self.address.clone(),
            https: self.use_https,
            debug: self.enable_debug,
        });
        controller.check_connection()?;
        controller.save_config().map_err(Into::into)
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Controller(#[from] thor::cli::Error),
    #[error(transparent)]
    Config(#[from] thor::cli::ConfigError),
    #[error(transparent)]
    Key(#[from] jcli_lib::key::Error),
}

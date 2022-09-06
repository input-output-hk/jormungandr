use super::wallet::Wallets;
use crate::cli::send::SendCommand;
use chain_crypto::digest::DigestOf;
use chain_impl_mockchain::{certificate::ExternalProposalId, testing::VoteTestGen};
use jormungandr_automation::jormungandr::RestError;
use jormungandr_lib::interfaces::VotePlan;
use serde_json::json;
use std::{fs, path::PathBuf};
use structopt::StructOpt;
use thiserror::Error;
use thor::cli::{CliController, Connection};
use typed_bytes::ByteBuilder;

///
///
/// Command line wallet for testing Jormungandr
///
#[derive(StructOpt, Debug)]
pub enum Command {
    /// Sets node rest API address. Verifies connection on set.
    Connect(Connect),
    /// Gets address of wallet in bech32 format
    Address,
    /// Prints wallet status (balance/spending counters/tokens)
    Status,
    /// Clears pending transactions to confirm. In case if expiration occured
    ClearTx,
    /// Confirms succesful transaction
    ConfirmTx,
    /// Pulls wallet data from the node
    Refresh,
    /// Prints pending or already sent fragments statuses
    Statuses,
    /// Sends fragments to nodes
    Send(SendCommand),
    /// Prints pending transactions (not confirmed)
    PendingTransactions,
    /// Allows to manage wallets: add/remove/select operations
    Wallets(Wallets),
    /// Utility command
    Utils(Utils),
    /// Rest api commands
    Rest(Rest),
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
            Command::Rest(rest) => rest.exec(controller),
            Command::Statuses => {
                print_delim();
                for (idx, (id, status)) in controller.statuses()?.iter().enumerate() {
                    println!("{}. {} -> {:#?}", idx, id, status);
                }
                print_delim();
                Ok(())
            }
            Command::Send(send) => send.exec(controller),
            Command::Utils(utils) => utils.exec(),
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

#[derive(StructOpt, Debug)]
pub enum Rest {
    VotePlans(VotePlans),
    Fragments,
}

impl Rest {
    pub fn exec(&self, controller: CliController) -> Result<(), Error> {
        match self {
            Self::Fragments => {
                println!("{:#?}", controller.fragment_logs()?);
                Ok(())
            }
            Self::VotePlans(vote_plans) => vote_plans.exec(controller),
        }
    }
}

#[derive(StructOpt, Debug)]
pub enum VotePlans {
    List,
}

impl VotePlans {
    pub fn exec(&self, controller: CliController) -> Result<(), Error> {
        match self {
            Self::List => {
                println!(
                    "{:#?}",
                    controller
                        .client()
                        .vote_plan_statuses()?
                        .iter()
                        .map(|x| x.id.to_string())
                        .collect::<Vec<String>>()
                );
                Ok(())
            }
        }
    }
}

#[derive(StructOpt, Debug)]
pub enum Utils {
    Examples(Examples),
    Decode(Decode),
}

impl Utils {
    pub fn exec(&self) -> Result<(), Error> {
        match self {
            Utils::Examples(examples) => examples.exec(),
            Utils::Decode(decode) => decode.exec(),
        }
    }
}

#[derive(StructOpt, Debug)]
pub enum Examples {
    VotePlan,
    Proposal,
}

impl Examples {
    pub fn exec(&self) -> Result<(), Error> {
        match self {
            Self::VotePlan => {
                let vote_plan: VotePlan = VoteTestGen::vote_plan().into();
                println!("{}", serde_json::to_string_pretty(&vote_plan)?);
                Ok(())
            }
            Self::Proposal => {
                let json = json!({
                    "category_name": "Fund 9",
                    "chain_vote_options": "blank,yes,no",
                    "challenge_id": "3",
                    "challenge_type": "simple",
                    "chain_vote_type": "private",
                    "internal_id": "0",
                    "proposal_funds": "22200",
                    "proposal_id": "423260",
                    "proposal_impact_score": "312",
                    "proposal_summary": "There is a lack a community engagement on twitter and many other social media platforms.",
                    "proposal_title": "Hard Fork Cafe",
                    "proposal_url": "https://cardano.ideascale.com/a/dtd/00000000",
                    "proposer_email": "example@mail",
                    "proposer_name": "hardforq, Q",
                    "proposer_relevant_experience": "Created a twitter thread in Fund 7 and 8 for Catalyst.",
                    "proposer_url": "https://twitter.com/Example",
                    "proposal_solution": "The Hard Fork creates information channels to increase communication and understanding of Catalyst."
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
                Ok(())
            }
        }
    }
}

#[derive(StructOpt, Debug)]
pub enum Decode {
    Proposal {
        #[structopt(short, long)]
        json: PathBuf,
    },
}

impl Decode {
    pub fn exec(&self) -> Result<(), Error> {
        match self {
            Self::Proposal { json } => {
                let json_as_string = serde_yaml::to_string(&fs::read_to_string(&json)?)?;
                let proposal_id: ExternalProposalId = DigestOf::digest_byteslice(
                    &ByteBuilder::new()
                        .bytes(json_as_string.as_bytes())
                        .finalize()
                        .as_byteslice(),
                );
                println!("{}", proposal_id);
                Ok(())
            }
        }
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
    #[error(transparent)]
    SerdeYaml(#[from] serde_yaml::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Rest(#[from] RestError),
}

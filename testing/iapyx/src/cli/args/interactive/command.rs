use super::WalletState;
use crate::cli::args::interactive::UserInteractionContoller;
use crate::Controller;
use bip39::Type;
use chain_addr::{AddressReadable, Discrimination};
use chain_impl_mockchain::fragment::FragmentId;
use jormungandr_testing_utils::testing::node::RestSettings;
use structopt::{clap::AppSettings, StructOpt};
use thiserror::Error;
use wallet_core::Choice;
#[derive(StructOpt, Debug)]
#[structopt(setting = AppSettings::NoBinaryName)]
pub enum IapyxCommand {
    /// recover wallet funds from mnemonic
    Recover(Recover),
    /// generate new wallet
    Generate(Generate),
    /// recover wallet funds from mnemonic
    Connect(Connect),
    /// Prints nodes related data, like stats,fragments etc.
    RetrieveFunds,
    /// Spawn leader or passive node (also legacy)
    Convert(Convert),
    /// confirms transaction
    ConfirmTx,
    /// Exit interactive mode
    Value,
    /// Prints wallets, nodes which can be used. Draw topology
    Status,
    /// Prints wallets, nodes which can be used. Draw topology
    Refresh,
    /// get Address
    Address(Address),
    /// send fragments
    // Vote(Vote),
    /// send fragments
    Logs,
    Exit,
    Proposals,
    Vote(Vote),
    Votes,
    IsConverted,
    PendingTransactions,
}

impl IapyxCommand {
    pub fn exec(&self, model: &mut UserInteractionContoller) -> Result<(), IapyxCommandError> {
        match self {
            IapyxCommand::PendingTransactions => {
                if let Some(controller) = model.controller.as_mut() {
                    let fragment_ids = controller
                        .pending_transactions()
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>();
                    println!("===================");
                    for (id, fragment_ids) in fragment_ids.iter().enumerate() {
                        println!("{}. {}", (id + 1), fragment_ids);
                    }
                    println!("===================");
                    return Ok(());
                }
                Err(IapyxCommandError::GeneralError(
                    "wallet not recovered or generated".to_string(),
                ))
            }
            IapyxCommand::IsConverted => {
                if let Some(controller) = model.controller.as_mut() {
                    println!("Is Converted: {}", controller.is_converted()?);
                    return Ok(());
                }
                Err(IapyxCommandError::GeneralError(
                    "wallet not recovered or generated".to_string(),
                ))
            }
            IapyxCommand::Votes => {
                if let Some(controller) = model.controller.as_mut() {
                    println!("===================");
                    for (id, vote) in controller.active_votes()?.iter().enumerate() {
                        println!("{}. {}", (id + 1), vote);
                    }
                    println!("===================");
                    return Ok(());
                }
                Err(IapyxCommandError::GeneralError(
                    "wallet not recovered or generated".to_string(),
                ))
            }
            IapyxCommand::Proposals => {
                if let Some(controller) = model.controller.as_mut() {
                    println!("===================");
                    for (id, proposal) in controller.get_proposals()?.iter().enumerate() {
                        println!(
                            "{}. #{} [{}] {}",
                            (id + 1),
                            proposal.chain_proposal_id_as_str(),
                            proposal.proposal_title,
                            proposal.proposal_summary
                        );
                        println!("{:#?}", proposal.chain_vote_options.0);
                    }
                    println!("===================");
                    return Ok(());
                }
                Err(IapyxCommandError::GeneralError(
                    "wallet not recovered or generated".to_string(),
                ))
            }
            IapyxCommand::Vote(vote) => vote.exec(model),
            IapyxCommand::ConfirmTx => {
                if let Some(controller) = model.controller.as_mut() {
                    controller.confirm_all_transactions();
                    return Ok(());
                }
                Err(IapyxCommandError::GeneralError(
                    "wallet not recovered or generated".to_string(),
                ))
            }
            IapyxCommand::Recover(recover) => recover.exec(model),
            IapyxCommand::Exit => Ok(()),
            IapyxCommand::Generate(generate) => generate.exec(model),
            IapyxCommand::Connect(connect) => connect.exec(model),
            IapyxCommand::RetrieveFunds => {
                if let Some(controller) = model.controller.as_mut() {
                    controller.retrieve_funds()?;
                    return Ok(());
                }
                Err(IapyxCommandError::GeneralError(
                    "wallet not recovered or generated".to_string(),
                ))
            }
            IapyxCommand::Convert(convert) => convert.exec(model),
            IapyxCommand::Value => {
                if let Some(controller) = model.controller.as_mut() {
                    println!("Total Value: {}", controller.total_value());
                    return Ok(());
                }
                Err(IapyxCommandError::GeneralError(
                    "wallet not recovered or generated".to_string(),
                ))
            }
            IapyxCommand::Status => {
                if let Some(controller) = model.controller.as_ref() {
                    let account_state = controller.get_account_state()?;
                    println!("-------------------------");
                    println!("- Delegation: {:?}", account_state.delegation());
                    println!("- Value: {}", account_state.value());
                    println!("- Spending counter: {}", account_state.counter());
                    println!("- Rewards: {:?}", account_state.last_rewards());
                    println!("--------------------------");
                    return Ok(());
                }
                Err(IapyxCommandError::GeneralError(
                    "wallet not recovered or generated".to_string(),
                ))
            }
            IapyxCommand::Refresh => {
                if let Some(controller) = model.controller.as_mut() {
                    controller.refresh_state()?;
                    return Ok(());
                }
                Err(IapyxCommandError::GeneralError(
                    "wallet not recovered or generated".to_string(),
                ))
            }
            IapyxCommand::Address(address) => address.exec(model),
            IapyxCommand::Logs => {
                if let Some(controller) = model.controller.as_mut() {
                    println!("{:#?}", controller.fragment_logs());
                    return Ok(());
                }
                Err(IapyxCommandError::GeneralError(
                    "wallet not recovered or generated".to_string(),
                ))
            }
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct Address {
    /// blocks execution until fragment is in block
    #[structopt(short = "t", long = "testing")]
    pub testing: bool,
}

impl Address {
    pub fn exec(&self, model: &mut UserInteractionContoller) -> Result<(), IapyxCommandError> {
        if let Some(controller) = model.controller.as_mut() {
            let (prefix, discrimination) = {
                if self.testing {
                    ("ca", Discrimination::Test)
                } else {
                    ("ta", Discrimination::Production)
                }
            };
            let address =
                AddressReadable::from_address(prefix, &controller.account(discrimination));
            println!("Address: {}", address.to_string());
            return Ok(());
        }
        Err(IapyxCommandError::GeneralError(
            "wallet not recovered or generated".to_string(),
        ))
    }
}

#[derive(StructOpt, Debug)]
pub struct Vote {
    /// choice
    #[structopt(short = "c", long = "choice")]
    pub choice: String,
    /// chain proposal id
    #[structopt(short = "p", long = "id")]
    pub proposal_id: String,
}

impl Vote {
    pub fn exec(&self, model: &mut UserInteractionContoller) -> Result<(), IapyxCommandError> {
        if let Some(controller) = model.controller.as_mut() {
            let proposals = controller.get_proposals()?;
            let proposal = proposals
                .iter()
                .find(|x| x.chain_proposal_id_as_str() == self.proposal_id)
                .ok_or_else(|| {
                    IapyxCommandError::GeneralError("Cannot find proposal".to_string())
                })?;
            let choice = proposal
                .chain_vote_options
                .0
                .get(&self.choice)
                .ok_or_else(|| IapyxCommandError::GeneralError("wrong choice".to_string()))?;
            controller.vote(proposal, Choice::new(*choice))?;
            return Ok(());
        }
        Err(IapyxCommandError::GeneralError(
            "wallet not recovered or generated".to_string(),
        ))
    }
}

#[derive(StructOpt, Debug)]
pub struct Convert {
    /// blocks execution until fragment is in block
    #[structopt(short = "w", long = "wait")]
    pub wait: bool,
}

impl Convert {
    pub fn exec(&self, model: &mut UserInteractionContoller) -> Result<(), IapyxCommandError> {
        if let Some(controller) = model.controller.as_mut() {
            controller.convert_and_send()?;
            if self.wait {
                println!("waiting for all pending transactions to be in block...");
                controller.wait_for_pending_transactions(std::time::Duration::from_secs(1))?;
            } else {
                println!(
                    "Conversion transactions ids: [{:?}]",
                    controller
                        .pending_transactions()
                        .iter()
                        .cloned()
                        .collect::<Vec<FragmentId>>()
                );
            }
            return Ok(());
        }
        Err(IapyxCommandError::GeneralError(
            "wallet not recovered or generated".to_string(),
        ))
    }
}

#[derive(StructOpt, Debug)]
pub struct Connect {
    #[structopt(short = "a", long = "address")]
    pub address: String,

    /// uses https for sending fragments
    #[structopt(short = "s", long = "use-https")]
    pub use_https_for_post: bool,

    /// uses https for sending fragments
    #[structopt(short = "d", long = "enable-debug")]
    pub enable_debug: bool,
}

impl Connect {
    pub fn exec(&self, model: &mut UserInteractionContoller) -> Result<(), IapyxCommandError> {
        let settings = RestSettings { 
            use_https_for_post: self.use_https_for_post, 
            enable_debug: self.enable_debug, 
            ..Default::default() 
        };

        if let Some(controller) = model.controller.as_mut() {
            controller.switch_backend(self.address.clone(), settings);
            return Ok(());
        }

        model.backend_address = self.address.clone();
        model.settings = settings;
        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct Recover {
    #[structopt(short = "m", long = "mnemonics")]
    pub mnemonics: Vec<String>,
}

impl Recover {
    pub fn exec(&self, model: &mut UserInteractionContoller) -> Result<(), IapyxCommandError> {
        model.controller = Some(Controller::recover(
            model.backend_address.clone(),
            &self.mnemonics.join(" "),
            &[],
            model.settings.clone(),
        )?);
        model.state = WalletState::Recovered;
        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct Generate {
    /// Words count
    #[structopt(short = "w", long = "words")]
    pub count: usize,
}

impl Generate {
    pub fn exec(&self, model: &mut UserInteractionContoller) -> Result<(), IapyxCommandError> {
        model.controller = Some(Controller::generate(
            model.backend_address.clone(),
            Type::from_word_count(self.count)?,
            model.settings.clone(),
        )?);
        model.state = WalletState::Generated;
        Ok(())
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Error, Debug)]
pub enum IapyxCommandError {
    #[error("{0}")]
    GeneralError(String),
    #[error("{0}")]
    ControllerError(#[from] crate::controller::ControllerError),
    #[error("wrong word count for generating wallet")]
    GenerateWalletError(#[from] bip39::Error),
}

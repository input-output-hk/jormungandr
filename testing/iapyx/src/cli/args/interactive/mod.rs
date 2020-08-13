pub mod command;

use crate::Controller;
pub use command::{IapyxCommand, IapyxCommandError};
use jormungandr_testing_utils::testing::node::RestSettings;
use jortestkit::prelude::{ConsoleWriter, InteractiveCommandError, InteractiveCommandExec};
use std::ffi::OsStr;
use structopt::StructOpt;

#[derive(Debug, Copy, Clone)]
pub enum WalletState {
    New,
    Recovered,
    Generated,
    FundsRetrieved,
}

pub struct IapyxInteractiveCommandExec {
    pub controller: UserInteractionContoller,
}

impl InteractiveCommandExec for IapyxInteractiveCommandExec {
    fn parse_and_exec(
        &mut self,
        tokens: Vec<String>,
        console: ConsoleWriter,
    ) -> std::result::Result<(), InteractiveCommandError> {
        match IapyxCommand::from_iter_safe(&mut tokens.iter().map(|x| OsStr::new(x))) {
            Ok(interactive) => {
                if let Err(err) = interactive.exec(&mut self.controller) {
                    console.format_error(InteractiveCommandError::UserError(err.to_string()));
                }
            }
            Err(err) => console.show_help(InteractiveCommandError::UserError(err.to_string())),
        }
        Ok(())
    }
}

pub struct UserInteractionContoller {
    pub state: WalletState,
    pub controller: Option<Controller>,
    pub backend_address: String,
    pub settings: RestSettings,
}

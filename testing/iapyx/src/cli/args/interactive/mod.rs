pub mod command;

use crate::Controller;
pub use command::{IapyxCommand, IapyxCommandError};
use console::Style;
use dialoguer::Input;
use std::ffi::OsStr;
use structopt::StructOpt;
use thiserror::Error;

#[derive(Debug, Copy, Clone)]
pub enum WalletState {
    New,
    Recovered,
    Generated,
    FundsRetrieved,
}

pub struct IapyxInteractiveCommandExec {
    controller: UserInteractionContoller
}

impl InteractiveCommandExec for IapyxInteractiveCommandExec {
    fn parse_and_exec(&mut self, tokens: Vec<String>, console: ConsoleWriter) -> std::result::Result<(),InteractiveCommandError> {
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
}
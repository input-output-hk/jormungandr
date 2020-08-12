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

lazy_static! {
    pub static ref ERROR_STYLE: Style = Style::new().red().bold();
    pub static ref SUCCESS_STYLE: Style = Style::new().green().bold();
    pub static ref INFO_STYLE: Style = Style::new().cyan().bold();
    pub static ref SCENARIO_TITLE_STYLE: Style = Style::new().bold();
}

pub struct UserInteraction {
    title: String,
    exit_phrase: String,
    command_prefix: String,
    pub state: WalletState,
    pub controller: Option<Controller>,
}

impl UserInteraction {
    pub fn new(title: String, exit_phrase: String, command_prefix: String) -> Self {
        Self {
            title,
            exit_phrase,
            command_prefix,
            state: WalletState::New,
            controller: None,
        }
    }

    pub fn interact(&mut self) -> Result<(), UserInteractionError> {
        self.show_info();
        loop {
            self.show_title();
            let tokens = self.read_line()?;
            if self.is_exit_command(&tokens) {
                return Ok(());
            }

            match IapyxCommand::from_iter_safe(&mut tokens.iter().map(|x| OsStr::new(x))) {
                Ok(interactive) => {
                    if let Err(err) = interactive.exec(self) {
                        println!("{}", ERROR_STYLE.apply_to(format!("Error: {:?}", err)));
                    }
                }
                Err(err) => self.print_help(Box::new(err)),
            }
        }
    }

    fn print_help(&self, error: Box<dyn std::error::Error>) {
        let message = format!("{}", error);
        //workaround for not showing app name
        println!(
            "{}",
            message.replace("iapyx-cli <SUBCOMMAND>", "<SUBCOMMAND>")
        );
    }

    fn show_title(&self) {
        self.show_text(&self.title);
    }

    fn show_text(&self, text: &str) {
        println!("{}", SUCCESS_STYLE.apply_to(text.to_string()));
    }

    fn show_info(&self) {
        println!("----------------------------------------------------------------");
        println!(
            "{}",
            SUCCESS_STYLE
                .apply_to("Welcome in iapyx, command line testing wallet for jormungandr.")
        );
        println!(
            "{}",
            SUCCESS_STYLE.apply_to("You can control each aspect of wallet:")
        );
        println!("{}", SUCCESS_STYLE.apply_to("- connect to backend,"));
        println!("{}", SUCCESS_STYLE.apply_to("- retrieve funds,"));
        println!("{}", SUCCESS_STYLE.apply_to("- convert wallet,"));
        println!(
            "{}",
            SUCCESS_STYLE.apply_to("- show wallet stats and pending fragments.")
        );
        println!();
        println!(
            "{}",
            SUCCESS_STYLE.apply_to("Type help for more informations.")
        );
        println!("----------------------------------------------------------------");
    }

    fn read_line(&self) -> Result<Vec<String>, UserInteractionError> {
        let input: String = Input::new()
            .with_prompt(&self.command_prefix)
            .interact()
            .unwrap();
        Ok(input
            .split_ascii_whitespace()
            .map(|x| x.to_owned())
            .collect())
    }

    fn is_exit_command(&self, tokens: &[String]) -> bool {
        tokens
            .first()
            .unwrap()
            .eq_ignore_ascii_case(&self.exit_phrase)
    }
}

impl Default for UserInteraction {
    fn default() -> UserInteraction {
        UserInteraction::new(
            "type command".to_string(),
            "exit".to_string(),
            ">".to_string(),
        )
    }
}

#[derive(Error, Debug)]
pub enum UserInteractionError {
    #[error("command error")]
    IapyxCommandError(#[from] IapyxCommandError),
}

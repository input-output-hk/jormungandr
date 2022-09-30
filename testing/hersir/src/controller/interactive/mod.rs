pub mod args;
mod command;
mod controller;

pub use command::InteractiveCommand;
pub use controller::{do_for_all_alias, UserInteractionController};
pub use jortestkit::prelude::{ConsoleWriter, InteractiveCommandError, InteractiveCommandExec};
use std::ffi::OsStr;
use structopt::StructOpt;
pub struct JormungandrInteractiveCommandExec {
    pub controller: UserInteractionController,
}

impl InteractiveCommandExec for JormungandrInteractiveCommandExec {
    fn parse_and_exec(
        &mut self,
        tokens: Vec<String>,
        console: ConsoleWriter,
    ) -> std::result::Result<(), InteractiveCommandError> {
        match InteractiveCommand::from_iter_safe(&mut tokens.iter().map(OsStr::new)) {
            Ok(interactive) => {
                if let Err(err) = {
                    match interactive {
                        InteractiveCommand::Show(show) => {
                            show.exec(&mut self.controller);
                            Ok(())
                        }
                        InteractiveCommand::Spawn(spawn) => spawn.exec(&mut self.controller),
                        InteractiveCommand::Exit => Ok(()),
                        InteractiveCommand::Describe(describe) => {
                            describe.exec(&mut self.controller)
                        }
                        InteractiveCommand::Send(send) => send.exec(&mut self.controller),
                        InteractiveCommand::Explorer(explorer) => {
                            explorer.exec(&mut self.controller)
                        }
                    }
                } {
                    console.format_error(InteractiveCommandError::UserError(err.to_string()));
                }
            }
            Err(err) => console.show_help(InteractiveCommandError::UserError(err.to_string())),
        }
        Ok(())
    }
}

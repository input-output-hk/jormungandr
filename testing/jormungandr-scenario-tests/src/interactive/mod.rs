pub mod args;

pub use crate::interactive::args::{InteractiveCommand, UserInteractionController};
use crate::{
    scenario::{repository::ScenarioResult, Context},
    test::Result,
};
use function_name::named;
use jortestkit::prelude::{
    ConsoleWriter, InteractiveCommandError, InteractiveCommandExec, UserInteraction,
};
use rand_chacha::ChaChaRng;
use std::ffi::OsStr;
use structopt::StructOpt;

#[named]
pub fn interactive(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            "Leader1",
            "Leader2" -> "Leader1",
            "Leader3" -> "Leader1",
            "Leader4" -> "Leader1",
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ "Leader2" ],
            initials = [
                "account" "unassigned1" with  500_000_000,
                "account" "unassigned2" with  100_000_000,
                "account" "delegated1" with 2_000_000_000 delegates to "Leader1",
                "account" "delegated2" with  300_000_000 delegates to "Leader2",
            ],
        }
    };

    let controller = scenario_settings.build(context).unwrap();
    let user_integration = jormungandr_user_interaction();

    let mut interactive_commands = JormungandrInteractiveCommandExec {
        controller: UserInteractionController::new(controller),
    };

    user_integration.interact(&mut interactive_commands)?;
    interactive_commands.tear_down();

    Ok(ScenarioResult::passed(name))
}

fn jormungandr_user_interaction() -> UserInteraction {
    UserInteraction::new(
        "jormungandr-scenario-tests".to_string(),
        "jormungandr interactive test".to_string(),
        "type command:".to_string(),
        "exit".to_string(),
        ">".to_string(),
        vec![
            "You can control each aspect of test:".to_string(),
            "- spawn nodes,".to_string(),
            "- send fragments,".to_string(),
            "- filter logs,".to_string(),
            "- show node stats and data.".to_string(),
        ],
    )
}

pub struct JormungandrInteractiveCommandExec {
    pub controller: UserInteractionController,
}

impl JormungandrInteractiveCommandExec {
    pub fn tear_down(self) {
        self.controller.finalize();
    }
}

impl InteractiveCommandExec for JormungandrInteractiveCommandExec {
    fn parse_and_exec(
        &mut self,
        tokens: Vec<String>,
        console: ConsoleWriter,
    ) -> std::result::Result<(), InteractiveCommandError> {
        match InteractiveCommand::from_iter_safe(&mut tokens.iter().map(|x| OsStr::new(x))) {
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

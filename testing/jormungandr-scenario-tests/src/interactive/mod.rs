mod args;
use crate::interactive::args::UserInteractionController;
use crate::{
    scenario::{repository::ScenarioResult, Context},
    test::Result,
};
pub use args::InteractiveCommand;
use jortestkit::prelude::{
    ConsoleWriter, InteractiveCommandError, InteractiveCommandExec, UserInteraction,
};
use rand_chacha::ChaChaRng;
use std::ffi::OsStr;
use structopt::StructOpt;

pub fn interactive(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "Testing the network",
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
                account "unassigned1" with  500_000_000,
                account "unassigned2" with  100_000_000,
                account "delegated1" with 2_000_000_000 delegates to "Leader1",
                account "delegated2" with  300_000_000 delegates to "Leader2",
            ],
        }
    };

    let mut controller = scenario_settings.build(context).unwrap();
    let user_integration = jormungandr_user_interaction();
    user_integration.interact(&mut JormungandrInteractiveCommandExec {
        controller: UserInteractionController::new(&mut controller),
    })?;
    controller.finalize();
    Ok(ScenarioResult::passed())
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

pub struct JormungandrInteractiveCommandExec<'a> {
    controller: UserInteractionController<'a>,
}

impl InteractiveCommandExec for JormungandrInteractiveCommandExec<'_> {
    fn parse_and_exec(
        &mut self,
        tokens: Vec<String>,
        console: ConsoleWriter,
    ) -> std::result::Result<(), InteractiveCommandError> {
        match InteractiveCommand::from_iter_safe(&mut tokens.iter().map(|x| OsStr::new(x))) {
            Ok(interactive) => {
                if let Err(err) = {
                    match interactive {
                        InteractiveCommand::Show(show) => show.exec(&mut self.controller),
                        InteractiveCommand::Spawn(spawn) => spawn.exec(&mut self.controller),
                        InteractiveCommand::Exit => Ok(()),
                        InteractiveCommand::Describe(describe) => {
                            describe.exec(&mut self.controller)
                        }
                        InteractiveCommand::Send(send) => send.exec(&mut self.controller),
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

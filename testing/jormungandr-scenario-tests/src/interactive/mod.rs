mod args;

use crate::{
    scenario::{repository::ScenarioResult, Controller},
    style,
    test::Result,
    Context,
};
pub use args::{InteractiveCommand, InteractiveCommandError};
use dialoguer::Input;
use jormungandr_testing_utils::wallet::Wallet;
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

    let user_integration: UserInteraction = Default::default();
    user_integration.interact(&mut controller)?;

    controller.finalize();
    Ok(ScenarioResult::passed())
}

#[derive(Debug)]
pub struct UserInteraction {
    title: String,
    exit_phrase: String,
    command_prefix: String,
}

impl UserInteraction {
    pub fn new(title: String, exit_phrase: String, command_prefix: String) -> Self {
        Self {
            title,
            exit_phrase,
            command_prefix,
        }
    }

    pub fn interact(&self, mut controller: &mut Controller) -> Result<()> {
        let mut wallets: Vec<Wallet> = controller.get_all_wallets();
        let mut nodes = vec![];
        let mut legacy_nodes = vec![];

        self.show_info();

        loop {
            self.show_title();

            let tokens = self.read_line()?;

            if self.is_exit_command(&tokens) {
                return Ok(());
            }

            match InteractiveCommand::from_iter_safe(&mut tokens.iter().map(|x| OsStr::new(x))) {
                Ok(interactive) => {
                    if let Err(err) = interactive.exec(
                        &mut controller,
                        &mut nodes,
                        &mut legacy_nodes,
                        &mut wallets,
                    ) {
                        println!("{}", style::error.apply_to(format!("Error: {}", err)));
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
            message.replace("jormungandr-scenario-tests <SUBCOMMAND>", "<SUBCOMMAND>")
        );
    }

    fn show_title(&self) {
        println!("{}", style::success.apply_to(self.title.to_string()));
    }

    fn show_info(&self) {
        println!("----------------------------------------------------------------");
        println!(
            "{}",
            style::success.apply_to("Welcome in jormungandr interactive test.")
        );
        println!(
            "{}",
            style::success.apply_to("You can control each aspect of test:")
        );
        println!("{}", style::success.apply_to("- spawn nodes,"));
        println!("{}", style::success.apply_to("- send fragments,"));
        println!("{}", style::success.apply_to("- filter logs,"));
        println!("{}", style::success.apply_to("- show node stats and data."));
        println!("");
        println!(
            "{}",
            style::success.apply_to("Type help for more informations.")
        );
        println!("----------------------------------------------------------------");
    }

    fn read_line(&self) -> Result<Vec<String>> {
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
        tokens.first().unwrap().eq_ignore_ascii_case("exit")
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

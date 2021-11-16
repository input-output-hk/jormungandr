use crate::controller::interactive::UserInteractionController;
use crate::controller::JormungandrInteractiveCommandExec;
use crate::{
    scenario::{repository::ScenarioResult, Context},
    test::Result,
};
use function_name::named;
use jortestkit::prelude::UserInteraction;

#[named]
pub fn interactive(context: Context) -> Result<ScenarioResult> {
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
        controller: UserInteractionController::new(controller.into()),
    };

    user_integration.interact(&mut interactive_commands)?;

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

use crate::controller::JormungandrInteractiveCommandExec;
use crate::controller::UserInteractionController;
use crate::{config::Config, error::Error};
use jormungandr_testing_utils::testing::network::{builder::NetworkBuilder, Topology};
use jortestkit::prelude::UserInteraction;

pub fn spawn_network(config: Config, topology: Topology) -> Result<(), Error> {
    let controller = NetworkBuilder::default()
        .topology(topology)
        .testing_directory(config.testing_directory())
        .blockchain_config(config.blockchain)
        .build()?;

    let user_integration = jormungandr_user_interaction();

    let mut interactive_commands = JormungandrInteractiveCommandExec {
        controller: UserInteractionController::new(controller),
    };

    user_integration
        .interact(&mut interactive_commands)
        .map_err(Into::into)
}

fn jormungandr_user_interaction() -> UserInteraction {
    UserInteraction::new(
        "hersir".to_string(),
        "interactive mode".to_string(),
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

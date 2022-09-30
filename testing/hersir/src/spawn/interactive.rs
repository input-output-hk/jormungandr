use crate::{
    builder::NetworkBuilder,
    config::Config,
    controller::{JormungandrInteractiveCommandExec, UserInteractionController},
    error::Error,
};
use jortestkit::prelude::UserInteraction;

pub fn spawn_network(config: Config) -> Result<(), Error> {
    let controller = NetworkBuilder::default()
        .topology(config.build_topology())
        .blockchain_config(config.build_blockchain())
        .session_settings(config.session)
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

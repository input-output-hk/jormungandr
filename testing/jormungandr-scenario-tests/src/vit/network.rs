use jortestkit::prelude::UserInteraction;
use crate::interactive::JormungandrInteractiveCommandExec;
use crate::vit::{WALLET_NODE,LEADER_1,LEADER_2,LEADER_3,LEADER_4};
use crate::{
    node::{LeadershipMode, PersistenceMode},
    scenario::{repository::ScenarioResult, Context},
    vit::QuickVitBackendSettingsBuilder,
    test::Result,
};
use rand_chacha::ChaChaRng;
use crate::interactive::UserInteractionController;
use jormungandr_testing_utils::testing::network_builder::SpawnParams;
use jormungandr_lib::interfaces::Explorer;

#[allow(unreachable_code)]
#[allow(clippy::empty_loop)]
pub fn vote_backend(mut context: Context<ChaChaRng>, quick_setup: QuickVitBackendSettingsBuilder, interactive: bool) -> Result<ScenarioResult> {
    
    let mut controller = quick_setup.build_settings(&mut context).build(context)?;

    // bootstrap network
    let leader_1 = controller.spawn_node_custom(
        SpawnParams::new(LEADER_1)
            .leader()
            .persistence_mode(PersistenceMode::Persistent)
            .explorer(Explorer { enabled: true }),
    )?;
    leader_1.wait_for_bootstrap()?;
    controller.monitor_nodes();

    //start bft node 2
    let leader_2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader_2.wait_for_bootstrap()?;

    //start bft node 3
    let leader_3 = controller.spawn_node(
        LEADER_3,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader_3.wait_for_bootstrap()?;

    //start bft node 4
    let leader_4 = controller.spawn_node(
        LEADER_4,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader_4.wait_for_bootstrap()?;

    // start passive node
    let wallet_node = controller.spawn_node_custom(
        SpawnParams::new(WALLET_NODE)
            .passive()
            .persistence_mode(PersistenceMode::Persistent)
            .explorer(Explorer { enabled: true }),
    )?;
    wallet_node.wait_for_bootstrap()?;

    // start proxy and vit station
    let vit_station = controller.spawn_vit_station()?;
    let wallet_proxy = controller.spawn_wallet_proxy(WALLET_NODE)?;

    match interactive {
        true => {
            let user_integration = vit_interaction();
            let mut interaction_controller = UserInteractionController::new(&mut controller);
            interaction_controller.proxies_mut().push(wallet_proxy);
            interaction_controller.vit_stations_mut().push(vit_station);

            user_integration.interact(&mut JormungandrInteractiveCommandExec {
                controller: UserInteractionController::new(&mut controller),
            })?;
            controller.finalize();       
        }
        false => loop {}
    }

    Ok(ScenarioResult::passed(""))
}


fn vit_interaction() -> UserInteraction {
    UserInteraction::new(
        "jormungandr-scenario-tests".to_string(),
        "jormungandr vit backend".to_string(),
        "type command:".to_string(),
        "exit".to_string(),
        ">".to_string(),
        vec![
            "You can control each aspect of backend:".to_string(),
            "- spawn nodes,".to_string(),
            "- send fragments,".to_string(),
            "- filter logs,".to_string(),
            "- show node stats and data.".to_string(),
        ],
    )
}

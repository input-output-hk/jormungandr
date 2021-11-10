use crate::{
    node::{LeadershipMode, PersistenceMode},
    test::{utils, Result},
    Context, ScenarioResult,
};
use function_name::named;
use jormungandr_lib::interfaces::Explorer;
use rand_chacha::ChaChaRng;
const LEADER_1: &str = "Leader_1";
const LEADER_2: &str = "Leader_2";
const LEADER_3: &str = "Leader_3";
const PASSIVE: &str = "Passive";

#[named]
pub fn passive_node_explorer(context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let name = function_name!();
    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER_1,
            LEADER_2 -> LEADER_1,
            LEADER_3 -> LEADER_1,
            PASSIVE -> LEADER_1,LEADER_2,LEADER_3
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                "account" "alice" with   500_000_000,
                "account" "bob" with  2_000_000_000 delegates to LEADER_1,
                "account" "clarice" with  2_000_000_000 delegates to LEADER_2,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    let mut leader_1 =
        controller.spawn_node(LEADER_1, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader_1.wait_for_bootstrap()?;

    controller.monitor_nodes();

    let mut leader_2 =
        controller.spawn_node(LEADER_2, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader_2.wait_for_bootstrap()?;

    let mut leader_3 =
        controller.spawn_node(LEADER_3, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader_3.wait_for_bootstrap()?;

    let mut passive = controller.spawn_node_custom(
        controller
            .new_spawn_params(PASSIVE)
            .passive()
            .in_memory()
            .explorer(Explorer { enabled: true }),
    )?;
    passive.wait_for_bootstrap()?;

    let mut alice = controller.wallet("alice")?;
    let bob = controller.wallet("bob")?;

    let mem_pool_check =
        controller
            .fragment_sender()
            .send_transaction(&mut alice, &bob, &leader_1, 1_000.into())?;

    // give some time to update explorer
    jortestkit::process::sleep(60);

    let transaction_id = passive
        .explorer()
        .transaction((*mem_pool_check.fragment_id()).into())?
        .data
        .unwrap()
        .transaction
        .id;
    utils::assert_equals(
        &transaction_id,
        &mem_pool_check.fragment_id().to_string(),
        "Wrong transaction id in explorer",
    )?;

    leader_1.shutdown()?;
    leader_2.shutdown()?;
    leader_3.shutdown()?;
    passive.shutdown()?;

    controller.finalize();
    Ok(ScenarioResult::passed(name))
}

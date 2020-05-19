use crate::{
    node::{LeadershipMode, PersistenceMode},
    test::{
        utils::{self, MeasurementReportInterval, SyncWaitParams},
        Result,
    },
    Context, ScenarioResult,
};
use jormungandr_testing_utils::testing::{FragmentNode, FragmentVerifier};
use rand_chacha::ChaChaRng;
use std::time::Duration;

const LEADER_1: &str = "Leader1";
const LEADER_2: &str = "Leader2";

pub fn two_transaction_to_two_leaders(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "L2101-Leader_to_leader_communication",
        &mut context,
        topology [
            LEADER_1 -> LEADER_2,
            LEADER_2
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1 ],
            initials = [
                account "delegated1" with  2_500_000_000 delegates to LEADER_2,
                account "delegated2" with  2_000_000_000 delegates to LEADER_1,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    let leader_1 =
        controller.spawn_node(LEADER_1, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let leader_2 =
        controller.spawn_node(LEADER_2, LeadershipMode::Leader, PersistenceMode::InMemory)?;

    controller.monitor_nodes();

    leader_2.wait_for_bootstrap()?;
    leader_1.wait_for_bootstrap()?;

    let mut wallet1 = controller.wallet("delegated2")?;
    let mut wallet2 = controller.wallet("delegated1")?;

    let fragment_sender = controller.fragment_sender();
    let fragment_verifier = FragmentVerifier;

    for _ in 0..10 {
        let check1 = fragment_sender.send_transaction(
            &mut wallet1,
            &wallet2,
            &leader_1 as &dyn FragmentNode,
            1_000.into(),
        )?;
        let check2 = fragment_sender.send_transaction(
            &mut wallet2,
            &wallet1,
            &leader_2 as &dyn FragmentNode,
            1_000.into(),
        )?;

        fragment_verifier.wait_and_verify_is_in_block(
            Duration::from_secs(2),
            check1,
            &leader_1 as &dyn FragmentNode,
        )?;
        fragment_verifier.wait_and_verify_is_in_block(
            Duration::from_secs(2),
            check2,
            &leader_2 as &dyn FragmentNode,
        )?;

        wallet1.confirm_transaction();
        wallet2.confirm_transaction();
    }

    utils::measure_and_log_sync_time(
        vec![&leader_1, &leader_2],
        SyncWaitParams::two_nodes().into(),
        "two_transaction_to_two_leaders_sync",
        MeasurementReportInterval::Standard,
    )?;

    leader_1.shutdown().unwrap();
    leader_2.shutdown().unwrap();
    controller.finalize();
    Ok(ScenarioResult::passed())
}

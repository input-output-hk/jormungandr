use crate::{
    node::{LeadershipMode, PersistenceMode},
    test::{
        utils::{self, MeasurementReportInterval, SyncWaitParams},
        Result,
    },
    Context, ScenarioResult,
};
use chain_impl_mockchain::block::BlockDate;
use jormungandr_testing_utils::testing::{
    node::time::wait_for_epoch, FragmentSenderSetup, FragmentVerifier, FragmentVerifierError,
};
use rand_chacha::ChaChaRng;
use std::time::Duration;

const ALICE: &str = "Alice";
const BOB: &str = " Bob";
const LEADER: &str = "Leader";
const PASSIVE: &str = "Passive";

pub fn no_expired_transactions_propagated(
    mut context: Context<ChaChaRng>,
) -> Result<ScenarioResult> {
    let name = "no_expired_transactions_propagated";

    let scenario_settings = prepare_scenario! {
        name,
        &mut context,
        topology [
            LEADER,
            PASSIVE -> LEADER,
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 30,
            slot_duration = 1,
            leaders = [LEADER],
            initials = [
                "account" ALICE with 1_000 delegates to LEADER,
                "account" BOB with 1_000 delegates to LEADER,
            ],
        }
    };

    let mut controller = scenario_settings.build(context).unwrap();

    let leader =
        controller.spawn_node(LEADER, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    let passive =
        controller.spawn_node(PASSIVE, LeadershipMode::Passive, PersistenceMode::InMemory)?;

    leader.wait_for_bootstrap()?;
    passive.wait_for_bootstrap()?;

    utils::measure_and_log_sync_time(
        &[&leader, &passive],
        SyncWaitParams::network_size(2, 1).into(),
        "no expired propagation sync",
        MeasurementReportInterval::Standard,
    )?;

    let mut alice = controller.wallet(ALICE)?;
    let bob = controller.wallet(BOB)?;

    let fragment_sender = controller.fragment_sender_with_setup(FragmentSenderSetup::no_verify());

    let mem_pool_check = fragment_sender.send_transaction_with_validity(
        &mut alice,
        &bob,
        &passive,
        100.into(),
        BlockDate {
            epoch: 0,
            slot_id: 0,
        },
    )?;

    FragmentVerifier::wait_and_verify_is_in_block(Duration::new(2, 0), mem_pool_check, &passive)?;

    wait_for_epoch(2, passive.rest());

    let mem_pool_check = fragment_sender.send_transaction_with_validity(
        &mut alice,
        &bob,
        &passive,
        100.into(),
        BlockDate {
            epoch: 0,
            slot_id: 0,
        },
    )?;

    matches!(
        FragmentVerifier::fragment_status(mem_pool_check, &passive),
        Err(FragmentVerifierError::FragmentNotInMemPoolLogs { .. }),
    );

    leader.shutdown()?;
    passive.shutdown()?;

    Ok(ScenarioResult::passed(name))
}

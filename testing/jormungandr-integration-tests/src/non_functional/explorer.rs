use super::NodeStuckError;
use crate::common::{
    jcli::JCli,
    jormungandr::{ConfigurationBuilder, JormungandrProcess},
    startup,
};

use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{ActiveSlotCoefficient, KESUpdateSpeed},
};
use jormungandr_testing_utils::{
    testing::{
        benchmark_consumption, benchmark_endurance, node::explorer::load::ExplorerRequestGen,
        Endurance, EnduranceBenchmarkRun, Thresholds,
    },
    wallet::Wallet,
};
use jortestkit::load::{Configuration, Monitor};
use std::{str::FromStr, time::Duration};

#[test]
pub fn test_explorer_is_in_sync_with_node_for_15_minutes() {
    let mut sender = startup::create_new_account_address();
    let mut receiver = startup::create_new_account_address();
    let jcli: JCli = Default::default();

    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        &[],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(4)
            .with_epoch_stability_depth(10)
            .with_kes_update_speed(KESUpdateSpeed::new(43200).unwrap()),
    )
    .unwrap();

    let output_value = 1 as u64;
    let benchmark = benchmark_endurance("test_explorer_is_in_sync_with_node_for_15_minutes")
        .target(Duration::from_secs(900))
        .start();

    let mut consumption_benchmark =
        benchmark_consumption("explorer with node is not consuming too much resources")
            .bare_metal_stake_pool_consumption_target()
            .for_process("Node with Explorer", jormungandr.pid() as usize)
            .start();

    loop {
        let transaction = jcli
            .transaction_builder(jormungandr.genesis_block_hash())
            .new_transaction()
            .add_account(&sender.address().to_string(), &output_value.into())
            .add_output(&receiver.address().to_string(), output_value.into())
            .finalize()
            .seal_with_witness_for_address(&sender)
            .to_message();

        sender.confirm_transaction();

        if let Err(err) =
            super::send_transaction_and_ensure_block_was_produced(&[transaction], &jormungandr)
        {
            let message = format!("{:?}", err);
            finish_test_prematurely(message, benchmark);
            return;
        }

        if let Err(err) = check_explorer_and_rest_are_in_sync(&jormungandr) {
            let message = format!("{:?}", err);
            finish_test_prematurely(message, benchmark);
            return;
        }

        if benchmark.max_endurance_reached() {
            benchmark.stop().print();
            return;
        }

        consumption_benchmark
            .snapshot()
            .expect("cannot gather system resources for snapshot");

        std::mem::swap(&mut sender, &mut receiver);
    }
}

fn finish_test_prematurely(error_message: String, benchmark: EnduranceBenchmarkRun) {
    // temporary threshold for the time issue with transaction stuck is resolved
    let temporary_threshold = Thresholds::<Endurance>::new_endurance(Duration::from_secs(400));
    benchmark
        .exception(error_message)
        .print_with_thresholds(temporary_threshold);
}

fn check_explorer_and_rest_are_in_sync(
    jormungandr: &JormungandrProcess,
) -> Result<(), NodeStuckError> {
    let jcli: JCli = Default::default();
    let block_tip = Hash::from_str(&jcli.rest().v0().tip(&jormungandr.rest_uri())).unwrap();

    let explorer = jormungandr.explorer();
    let block = explorer
        .last_block()
        .map_err(NodeStuckError::InternalExplorerError)?
        .data
        .unwrap()
        .tip
        .block;

    if block_tip == Hash::from_str(&block.id).unwrap() {
        Ok(())
    } else {
        Err(NodeStuckError::ExplorerTipIsOutOfSync {
            actual: Hash::from_str(&block.id).unwrap(),
            expected: block_tip,
            logs: jormungandr.logger.get_log_content(),
        })
    }
}

#[test]
pub fn explorer_load_test() {
    let stake_pool_owners: Vec<Wallet> =
        std::iter::from_fn(|| Some(startup::create_new_account_address()))
            .take(100)
            .collect();
    let addresses: Vec<Wallet> = std::iter::from_fn(|| Some(startup::create_new_account_address()))
        .take(100)
        .collect();

    let (jormungandr, _) = startup::start_stake_pool(
        &stake_pool_owners,
        &addresses,
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(4)
            .with_epoch_stability_depth(10)
            .with_kes_update_speed(KESUpdateSpeed::new(43200).unwrap()),
    )
    .unwrap();

    let mut request_gen = ExplorerRequestGen::new(jormungandr.explorer());
    request_gen
        .do_setup(addresses.iter().map(|x| x.address().to_string()).collect())
        .unwrap();
    let config = Configuration::duration(
        100,
        std::time::Duration::from_secs(60),
        100,
        Monitor::Progress(100),
        0,
    );
    let stats = jortestkit::load::start_sync(request_gen, config, "Explorer load test");
    assert!((stats.calculate_passrate() as u32) > 95);
}

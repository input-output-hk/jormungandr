use super::NodeStuckError;
use crate::startup;
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{
        explorer::configuration::ExplorerParams, ConfigurationBuilder, ExplorerProcess,
        JormungandrProcess,
    },
    testing::{
        benchmark_consumption, benchmark_endurance, Endurance, EnduranceBenchmarkRun, Thresholds,
    },
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{ActiveSlotCoefficient, BlockDate, KesUpdateSpeed},
};
use jortestkit::load::{ConfigurationBuilder as LoadConfigurationBuilder, Monitor};
use mjolnir::generators::ExplorerRequestGen;
use std::{str::FromStr, time::Duration};
use thor::{BlockDateGenerator, Wallet};

#[test]
pub fn test_explorer_is_in_sync_with_node_for_15_minutes() {
    let mut sender = thor::Wallet::default();
    let mut receiver = thor::Wallet::default();
    let jcli: JCli = Default::default();

    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        &[],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(4)
            .with_epoch_stability_depth(10)
            .with_kes_update_speed(KesUpdateSpeed::new(43200).unwrap()),
    )
    .unwrap();
    let explorer_process = jormungandr.explorer(ExplorerParams::default()).unwrap();

    let output_value = 1_u64;
    let benchmark = benchmark_endurance("test_explorer_is_in_sync_with_node_for_15_minutes")
        .target(Duration::from_secs(900))
        .start();

    let mut consumption_benchmark =
        benchmark_consumption("explorer with node is not consuming too much resources")
            .bare_metal_stake_pool_consumption_target()
            .for_process("Node with Explorer", jormungandr.pid() as usize)
            .start();

    let shift = BlockDate::new(0, 4);
    let settings = jormungandr.rest().settings().unwrap();
    let expiry_block_date_generator = BlockDateGenerator::rolling(&settings, shift.into(), false);

    loop {
        let transaction = jcli
            .transaction_builder(jormungandr.genesis_block_hash())
            .new_transaction()
            .add_account(&sender.address().to_string(), &output_value.into())
            .add_output(&receiver.address().to_string(), output_value.into())
            .set_expiry_date(expiry_block_date_generator.block_date().into())
            .finalize()
            .seal_with_witness_data(sender.witness_data())
            .to_message();

        sender.confirm_transaction();

        if let Err(err) =
            super::send_transaction_and_ensure_block_was_produced(&[transaction], &jormungandr)
        {
            let message = format!("{:?}", err);
            finish_test_prematurely(message, benchmark);
            return;
        }

        if let Err(err) = check_explorer_and_rest_are_in_sync(&jormungandr, &explorer_process) {
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
    explorer_process: &ExplorerProcess,
) -> Result<(), NodeStuckError> {
    let jcli: JCli = Default::default();
    let block_tip = Hash::from_str(&jcli.rest().v0().tip(&jormungandr.rest_uri())).unwrap();

    let explorer = explorer_process.client();
    let last_block = explorer
        .last_block()
        .map_err(NodeStuckError::InternalExplorerError)?;

    if block_tip == Hash::from_str(&last_block.block().id).unwrap() {
        Ok(())
    } else {
        Err(NodeStuckError::ExplorerTipIsOutOfSync {
            actual: Hash::from_str(&last_block.block().id).unwrap(),
            expected: block_tip,
            logs: jormungandr.logger.get_log_content(),
        })
    }
}

#[test]
pub fn explorer_load_test() {
    let stake_pool_owners: Vec<Wallet> = std::iter::from_fn(|| Some(thor::Wallet::default()))
        .take(100)
        .collect();
    let addresses: Vec<Wallet> = std::iter::from_fn(|| Some(thor::Wallet::default()))
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
            .with_kes_update_speed(KesUpdateSpeed::new(43200).unwrap()),
    )
    .unwrap();
    let explorer = jormungandr.explorer(ExplorerParams::default()).unwrap();

    let mut request_gen = ExplorerRequestGen::new(explorer.client().clone());
    request_gen
        .do_setup(addresses.iter().map(|x| x.address().to_string()).collect())
        .unwrap();
    let config = LoadConfigurationBuilder::duration(Duration::from_secs(60))
        .thread_no(30)
        .step_delay(Duration::from_millis(100))
        .monitor(Monitor::Progress(100))
        .status_pace(Duration::from_secs(1))
        .build();
    let stats = jortestkit::load::start_sync(request_gen, config, "Explorer load test");
    assert!((stats.calculate_passrate() as u32) > 95);
}

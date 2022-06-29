use crate::startup;
use chain_impl_mockchain::block::BlockDate;
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::ConfigurationBuilder,
    testing::{benchmark_consumption, benchmark_endurance, ResourcesUsage},
};
use jormungandr_lib::interfaces::ActiveSlotCoefficient;
use jortestkit::process as process_utils;
use std::time::Duration;
use thor::TransactionHash;

#[test]
pub fn collect_reward_for_15_minutes() {
    let jcli: JCli = Default::default();
    let duration_48_hours = Duration::from_secs(900);

    let mut sender = thor::Wallet::default();
    let receiver = thor::Wallet::default();

    let stake_pool_owners = [
        sender.clone(),
        receiver.clone(),
        thor::Wallet::default(),
        thor::Wallet::default(),
        thor::Wallet::default(),
        thor::Wallet::default(),
        thor::Wallet::default(),
        thor::Wallet::default(),
    ];
    let (jormungandr, _stake_pool_ids) = startup::start_stake_pool(
        &stake_pool_owners,
        &[],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(20)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(3),
    )
    .unwrap();

    let benchmark_endurance = benchmark_endurance("collect_reward_for_15_minutes")
        .target(duration_48_hours)
        .start();

    let mut benchmark_consumption =
        benchmark_consumption("collect_reward_for_15_minutes_resources")
            .target(ResourcesUsage::new(10, 200_000, 5_000_000))
            .for_process("Node 15 minutes up", jormungandr.pid() as usize)
            .start();

    loop {
        let new_transaction = thor::FragmentBuilder::new(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            BlockDate::first().next_epoch(),
        )
        .transaction(&sender, receiver.address(), 10.into())
        .unwrap()
        .encode();

        jcli.rest()
            .v0()
            .message()
            .post(&new_transaction, jormungandr.rest_uri());
        sender.confirm_transaction();

        benchmark_consumption.snapshot().unwrap();

        if benchmark_endurance.max_endurance_reached() {
            benchmark_consumption.stop().print();
            benchmark_endurance.stop().print();
            return;
        }

        if let Err(err) = jormungandr.check_no_errors_in_log() {
            let message = format!("{}", err);
            benchmark_endurance.exception(message.clone()).print();
            benchmark_consumption.exception(message.clone()).print();
            std::panic::panic_any(message);
        }

        benchmark_consumption.snapshot().unwrap();
        if benchmark_endurance.max_endurance_reached() {
            benchmark_consumption.stop().print();
            benchmark_endurance.stop().print();
            return;
        }
        process_utils::sleep(5);

        let _rewards = jormungandr
            .rest()
            .reward_history(1)
            .expect("failed to get last reward");
    }
}

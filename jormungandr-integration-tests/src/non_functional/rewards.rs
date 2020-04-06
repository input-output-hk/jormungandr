use crate::common::{
    file_utils, jcli_wrapper, jormungandr::ConfigurationBuilder, process_utils, startup,
    transaction_utils::TransactionHash,
};

use jormungandr_lib::{
    interfaces::ActiveSlotCoefficient,
    testing::{benchmark_consumption, benchmark_endurance, ResourcesUsage},
};
use std::time::Duration;

#[test]
pub fn collect_reward_for_15_minutes() {
    let duration_48_hours = Duration::from_secs(900);

    let path = file_utils::get_path_in_temp("rewards_dump");
    std::env::set_var("JORMUNGANDR_REWARD_DUMP_DIRECTORY", path.to_str().unwrap());

    let mut sender = startup::create_new_account_address();
    let receiver = startup::create_new_account_address();

    let stake_pool_owners = [
        sender.clone(),
        receiver.clone(),
        startup::create_new_account_address(),
        startup::create_new_account_address(),
        startup::create_new_account_address(),
        startup::create_new_account_address(),
        startup::create_new_account_address(),
        startup::create_new_account_address(),
    ];
    let (jormungandr, stake_pool_ids) = startup::start_stake_pool(
        &stake_pool_owners,
        &[],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(20)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(3),
    )
    .unwrap();

    let benchmark_endurance = benchmark_endurance("collect_reward_for_15_minutes")
        .target(duration_48_hours.clone())
        .start();

    let mut benchmark_consumption =
        benchmark_consumption("collect_reward_for_15_minutes_resources")
            .target(ResourcesUsage::new(10, 200_000, 5_000_000))
            .for_process(jormungandr.pid() as usize)
            .start();

    loop {
        let new_transaction = sender
            .transaction_to(
                &jormungandr.genesis_block_hash(),
                &jormungandr.fees(),
                receiver.address(),
                10.into(),
            )
            .unwrap()
            .encode();

        jcli_wrapper::assert_post_transaction(&new_transaction, &jormungandr.rest_address());
        sender.confirm_transaction();

        benchmark_consumption.snapshot();

        if benchmark_endurance.max_endurance_reached() {
            benchmark_consumption.stop().print();
            benchmark_endurance.stop().print();
            return;
        }

        if let Err(err) = jormungandr.check_no_errors_in_log() {
            let message = format!("{}", err);
            benchmark_endurance.exception(message.clone()).print();
            benchmark_consumption.exception(message.clone()).print();
            panic!(message.clone());
        }

        benchmark_consumption.snapshot();
        if benchmark_endurance.max_endurance_reached() {
            benchmark_consumption.stop().print();
            benchmark_endurance.stop().print();
            return;
        }
        process_utils::sleep(5);
    }
}

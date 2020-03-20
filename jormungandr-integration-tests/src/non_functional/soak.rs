#![cfg(feature = "soak-non-functional")]
use crate::common::{
    jcli_wrapper::{self, jcli_transaction_wrapper::JCLITransactionWrapper},
    jormungandr::ConfigurationBuilder,
    process_utils::Wait,
    startup,
    transaction_utils::TransactionHash,
};

use jormungandr_lib::{
    interfaces::{ActiveSlotCoefficient, KESUpdateSpeed, Mempool},
    testing::{benchmark_consumption, benchmark_endurance},
};
use std::time::Duration;

#[test]
pub fn test_blocks_are_being_created_for_48_hours() {
    let duration_48_hours = Duration::from_secs(60);

    let mut receiver = startup::create_new_account_address();
    let mut sender = startup::create_new_account_address();
    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        &[],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(20)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(3)
            .with_kes_update_speed(KESUpdateSpeed::new(43200).unwrap())
            .with_mempool(Mempool {
                pool_max_entries: 1_000_000usize.into(),
                log_max_entries: 1_000_000usize.into(),
            }),
    )
    .unwrap();

    let benchmark_endurance = benchmark_endurance("test_blocks_are_being_created_for_48_hours")
        .target(duration_48_hours.clone())
        .start();

    let mut benchmark_consumption =
        benchmark_consumption("test_blocks_are_being_created_for_48_hours_resources")
            .bare_metal_stake_pool_consumption_target()
            .for_process(jormungandr.pid() as usize)
            .start();

    loop {
        let new_transaction = sender
            .transaction_to(
                &jormungandr.genesis_block_hash(),
                &jormungandr.fees(),
                receiver.address(),
                1.into(),
            )
            .unwrap()
            .encode();

        let wait: Wait = Wait::new(Duration::from_secs(10), 10);
        let fragment_id =
            jcli_wrapper::assert_post_transaction(&new_transaction, &jormungandr.rest_address());
        if let Err(err) =
            jcli_wrapper::wait_until_transaction_processed(fragment_id.clone(), &jormungandr, &wait)
        {
            let message = format!("error: {}, transaction with id: {} was not in a block as expected. Message log: {:?}. Jormungandr log: {}", 
                err,
                fragment_id,
                jcli_wrapper::assert_get_rest_message_log(&jormungandr.rest_address()),
                jormungandr.logger.get_log_content()
            );
            benchmark_endurance.exception(message.clone()).print();
            benchmark_consumption.exception(message.clone()).print();
            panic!(message.clone());
        }
        sender.confirm_transaction();

        benchmark_consumption.snapshot();

        if benchmark_endurance.max_endurance_reached() {
            benchmark_consumption.stop().print();
            benchmark_endurance.stop().print();
            return;
        }

        std::mem::swap(&mut sender, &mut receiver);
    }
}

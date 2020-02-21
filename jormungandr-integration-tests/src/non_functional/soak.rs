#![cfg(feature = "soak-non-functional")]
use crate::common::{
    jcli_wrapper::{self, jcli_transaction_wrapper::JCLITransactionWrapper},
    jormungandr::ConfigurationBuilder,
    process_utils::Wait,
    startup,
};

use jormungandr_lib::interfaces::{ActiveSlotCoefficient, KESUpdateSpeed, Mempool};
use std::time::{Duration, SystemTime};

#[test]
pub fn test_blocks_are_being_created_for_48_hours() {
    let duration_48_hours = Duration::from_secs(86400 * 2);

    let mut receiver = startup::create_new_account_address();
    let mut sender = startup::create_new_account_address();
    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
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

    let now = SystemTime::now();
    loop {
        let new_transaction =
            JCLITransactionWrapper::new_transaction(&jormungandr.config.genesis_block_hash)
                .assert_add_account(&sender.address().to_string(), &1.into())
                .assert_add_output(&receiver.address().to_string(), &1.into())
                .assert_finalize()
                .seal_with_witness_for_address(&sender)
                .assert_to_message();

        let wait: Wait = Wait::new(Duration::from_secs(10), 10);
        let fragment_id =
            jcli_wrapper::assert_post_transaction(&new_transaction, &jormungandr.rest_address());
        if let Err(err) =
            jcli_wrapper::wait_until_transaction_processed(fragment_id.clone(), &jormungandr, &wait)
        {
            panic!(format!("error: {}, transaction with id: {} was not in a block as expected. Message log: {:?}. Jormungandr log: {}", 
                err,
                fragment_id,
                jcli_wrapper::assert_get_rest_message_log(&jormungandr.rest_address()),
                jormungandr.logger.get_log_content()
            ));
        }
        sender.confirm_transaction();

        if now.elapsed().unwrap() > duration_48_hours.clone() {
            break;
        }

        std::mem::swap(&mut sender, &mut receiver);
    }
}

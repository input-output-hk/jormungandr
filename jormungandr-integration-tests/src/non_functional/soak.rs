#![cfg(feature = "soak-non-functional")]
use crate::common::{
    configuration::genesis_model::Fund,
    data::address::Account,
    jcli_wrapper::{self, jcli_transaction_wrapper::JCLITransactionWrapper},
    jormungandr::{ConfigurationBuilder, Starter},
    process_utils::Wait,
    startup,
};

use jormungandr_lib::interfaces::UTxOInfo;
use std::iter;
use std::time::SystemTime;

#[test]
pub fn test_blocks_are_being_created_for_48_hours() {
    let mut receiver = startup::create_new_account_address();
    let mut sender = startup::create_new_account_address();
    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(20)
            .with_consensus_genesis_praos_active_slot_coeff("0.999")
            .with_slot_duration(3)
            .with_kes_update_speed(43200),
    )
    .unwrap();

    let now = SystemTime::now();
    loop {
        let new_transaction =
            JCLITransactionWrapper::new_transaction(&jormungandr.config.genesis_block_hash)
                .assert_add_account(&sender.address, &1.into())
                .assert_add_output(&receiver.address, &1.into())
                .assert_finalize()
                .seal_with_witness_for_address(&sender)
                .assert_to_message();

        let wait: Wait = Default::default();
        let fragment_id =
            jcli_wrapper::assert_post_transaction(&new_transaction, &jormungandr.rest_address());
        if let Err(err) = jcli_wrapper::wait_until_transaction_processed(
            fragment_id.clone(),
            &jormungandr.rest_address(),
            &wait,
        ) {
            panic!(format!("error: {}, transaction with id: {} was not in a block as expected. Message log: {:?}. Jormungandr log: {}", 
                err,
                fragment_id,
                jcli_wrapper::assert_get_rest_message_log(&jormungandr.rest_address()),
                jormungandr.logger.get_log_content()
            ));
        }
        sender.confirm_transaction();

        // 48 hours
        if now.elapsed().unwrap().as_secs() > (86400 * 2) {
            break;
        }

        std::mem::swap(&mut sender, &mut receiver);
    }
}

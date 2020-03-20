#![cfg(feature = "sanity-non-functional")]
use super::NodeStuckError;
use crate::common::{
    jcli_wrapper::{self, jcli_transaction_wrapper::JCLITransactionWrapper},
    jormungandr::{ConfigurationBuilder, JormungandrProcess},
    startup,
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{ActiveSlotCoefficient, KESUpdateSpeed, Value},
    testing::{
        benchmark_efficiency, benchmark_endurance, Endurance, EnduranceBenchmarkRun, Thresholds,
    },
    wallet::Wallet,
};
use std::{iter, str::FromStr, time::Duration};

#[test]
pub fn test_explorer_is_in_sync_with_node_for_15_minutes() {
    let mut sender = startup::create_new_account_address();
    let mut receiver = startup::create_new_account_address();

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

    loop {
        let transaction =
            JCLITransactionWrapper::new_transaction(&jormungandr.config.genesis_block_hash)
                .assert_add_account(&sender.address().to_string(), &output_value.into())
                .assert_add_output(&receiver.address().to_string(), &output_value.into())
                .assert_finalize()
                .seal_with_witness_for_address(&sender)
                .assert_to_message();

        sender.confirm_transaction();

        if let Err(err) =
            super::send_transaction_and_ensure_block_was_produced(&vec![transaction], &jormungandr)
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
    let block_tip = Hash::from_str(&jcli_wrapper::assert_rest_get_block_tip(
        &jormungandr.rest_address(),
    ))
    .unwrap();

    let explorer = jormungandr.explorer();
    let block = explorer
        .get_last_block()
        .map_err(|e| NodeStuckError::InternalExplorerError(e))?;

    match block_tip == block.id() {
        true => Ok(()),
        false => Err(NodeStuckError::ExplorerTipIsOutOfSync {
            actual: block.id(),
            expected: block_tip,
            logs: jormungandr.logger.get_log_content(),
        }),
    }
}

pub mod soak;
pub mod stress;

use crate::common::jcli_wrapper;
use crate::common::jormungandr::JormungandrProcess;

pub fn send_transaction_and_ensure_block_was_produced(
    transation_messages: &Vec<String>,
    jormungandr: &JormungandrProcess,
) {
    let host = &jormungandr.config.get_node_address();
    let block_tip_before_transaction = jcli_wrapper::assert_rest_get_block_tip(host);
    let block_counter_before_transaction = jormungandr.logger.get_created_blocks_counter();

    jcli_wrapper::assert_all_transactions_in_block(&transation_messages, host);

    let block_tip_after_transaction = jcli_wrapper::assert_rest_get_block_tip(host);
    let block_counter_after_transaction = jormungandr.logger.get_created_blocks_counter();

    let is_block_tip_different = block_tip_before_transaction != block_tip_after_transaction;
    let is_block_counter_increased =
        block_counter_before_transaction < block_counter_after_transaction;

    assert!(
        is_block_tip_different,
        "Node stopped producing blocks. Block tip is still the same after transaction\
         was put in the block {} vs {}. Logs: {}",
        block_tip_before_transaction,
        block_tip_after_transaction,
        jormungandr.logger.get_log_content()
    );

    jormungandr.assert_no_errors_in_log();
    assert!(
        is_block_counter_increased,
        "Node stopped producing blocks. No new entry about producing block in logs, counter stopped at {}. Logs: {}",
        block_counter_before_transaction,
        jormungandr.logger.get_log_content()
    );
}

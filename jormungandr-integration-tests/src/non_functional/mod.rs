/*
 Explorer soak test. Run node for ~15 minutes and verify explorer is in sync with node rest
*/
pub mod explorer;
/*
 Sanity performance tests. Quick tests to check overall node performance.
 Run some transaction for ~15 minutes or specified no of transactions (100)
*/
pub mod sanity;
/*
Long running test for self node (48 h)
*/
pub mod soak;

use crate::common::{
    explorer::ExplorerError,
    jcli_wrapper,
    jormungandr::{JormungandrError, JormungandrProcess},
    process_utils,
};
use jormungandr_lib::{crypto::hash::Hash, interfaces::Value, wallet::Wallet};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NodeStuckError {
    #[error("node tip is not moving up. Stuck at {tip_hash} ")]
    TipIsNotMoving { tip_hash: String, logs: String },
    #[error("node block counter is not moving up. Stuck at {block_counter}")]
    BlockCounterIsNoIncreased { block_counter: u64, logs: String },
    #[error("accounts funds were not trasfered (actual: {actual} vs expected: {expected}). Logs: {logs}")]
    FundsNotTransfered {
        actual: Value,
        expected: Value,
        logs: String,
    },
    #[error("explorer is out of sync with rest node (actual: {actual} vs expected: {expected}). Logs: {logs}")]
    ExplorerTipIsOutOfSync {
        actual: Hash,
        expected: Hash,
        logs: String,
    },
    #[error("error in logs found")]
    InternalJormungandrError(#[from] JormungandrError),
    #[error("jcli error")]
    InternalJcliError(#[from] jcli_wrapper::Error),
    #[error("exploer error")]
    InternalExplorerError(#[from] ExplorerError),
}

pub fn send_transaction_and_ensure_block_was_produced(
    transation_messages: &Vec<String>,
    jormungandr: &JormungandrProcess,
) -> Result<(), NodeStuckError> {
    let block_tip_before_transaction =
        jcli_wrapper::assert_rest_get_block_tip(&jormungandr.rest_address());
    let block_counter_before_transaction = jormungandr.logger.get_created_blocks_counter();

    jcli_wrapper::send_transactions_and_wait_until_in_block(&transation_messages, &jormungandr)
        .map_err(|err| NodeStuckError::InternalJcliError(err))?;

    let block_tip_after_transaction =
        jcli_wrapper::assert_rest_get_block_tip(&jormungandr.rest_address());
    let block_counter_after_transaction = jormungandr.logger.get_created_blocks_counter();

    if block_tip_before_transaction == block_tip_after_transaction {
        return Err(NodeStuckError::TipIsNotMoving {
            tip_hash: block_tip_after_transaction.clone(),
            logs: jormungandr.logger.get_log_content(),
        });
    }

    if block_counter_before_transaction == block_counter_after_transaction {
        return Err(NodeStuckError::BlockCounterIsNoIncreased {
            block_counter: block_counter_before_transaction as u64,
            logs: jormungandr.logger.get_log_content(),
        });
    }

    Ok(())
}

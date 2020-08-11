#![cfg(feature = "sanity-non-functional")]

pub mod compatibility;
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

/*
Long running test for dumping rewards each epoch
*/
pub mod rewards;

use crate::common::{
    jcli_wrapper,
    jormungandr::{JormungandrError, JormungandrProcess},
};
use jormungandr_lib::{crypto::hash::Hash, interfaces::Value};
use jormungandr_testing_utils::{testing::node::ExplorerError, wallet::Wallet};
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
    transation_messages: &[String],
    jormungandr: &JormungandrProcess,
) -> Result<(), NodeStuckError> {
    let block_tip_before_transaction =
        jcli_wrapper::assert_rest_get_block_tip(&jormungandr.rest_uri());
    let block_counter_before_transaction = jormungandr.logger.get_created_blocks_counter();

    jcli_wrapper::send_transactions_and_wait_until_in_block(&transation_messages, &jormungandr)
        .map_err(NodeStuckError::InternalJcliError)?;

    let block_tip_after_transaction =
        jcli_wrapper::assert_rest_get_block_tip(&jormungandr.rest_uri());
    let block_counter_after_transaction = jormungandr.logger.get_created_blocks_counter();

    if block_tip_before_transaction == block_tip_after_transaction {
        return Err(NodeStuckError::TipIsNotMoving {
            tip_hash: block_tip_after_transaction,
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

pub fn check_transaction_was_processed(
    transaction: String,
    receiver: &Wallet,
    value: u64,
    jormungandr: &JormungandrProcess,
) -> Result<(), NodeStuckError> {
    send_transaction_and_ensure_block_was_produced(&[transaction], &jormungandr)?;

    check_funds_transferred_to(&receiver.address().to_string(), value.into(), &jormungandr)?;

    jormungandr
        .check_no_errors_in_log()
        .map_err(NodeStuckError::InternalJormungandrError)
}

pub fn check_funds_transferred_to(
    address: &str,
    value: Value,
    jormungandr: &JormungandrProcess,
) -> Result<(), NodeStuckError> {
    let account_state =
        jcli_wrapper::assert_rest_account_get_stats(address, &jormungandr.rest_uri());

    if *account_state.value() != value {
        return Err(NodeStuckError::FundsNotTransfered {
            actual: *account_state.value(),
            expected: value,
            logs: jormungandr.logger.get_log_content(),
        });
    }
    Ok(())
}

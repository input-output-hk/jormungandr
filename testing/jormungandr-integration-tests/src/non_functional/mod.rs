#[cfg(feature = "sanity-non-functional")]
pub mod compatibility;
/*
 Explorer soak test. Run node for ~15 minutes and verify explorer is in sync with node rest
*/
#[cfg(feature = "sanity-non-functional")]
pub mod explorer;
/*
 Sanity performance tests. Quick tests to check overall node performance.
 Run some transaction for ~15 minutes or specified no of transactions (100)
*/
#[cfg(feature = "sanity-non-functional")]
pub mod transaction;
/*
Long running test for self node (48 h)
*/
#[cfg(feature = "soak-non-functional")]
pub mod soak;

/*
  Quick load test for rest api
*/
#[cfg(feature = "sanity-non-functional")]
pub mod rest;

/*
Long running test for dumping rewards each epoch
*/
#[cfg(feature = "sanity-non-functional")]
pub mod rewards;

#[cfg(feature = "sanity-non-functional")]
pub mod fragment;

#[cfg(feature = "sanity-non-functional")]
pub mod bootstrap;

#[cfg(feature = "sanity-non-functional")]
pub mod persistent_log;

pub mod voting;

use crate::common::{
    jcli::{self, JCli},
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
    InternalJcliError(#[from] jcli::Error),
    #[error("exploer error")]
    InternalExplorerError(#[from] ExplorerError),
}

pub fn send_transaction_and_ensure_block_was_produced(
    transation_messages: &[String],
    jormungandr: &JormungandrProcess,
) -> Result<(), NodeStuckError> {
    let jcli: JCli = Default::default();
    let block_tip_before_transaction = jcli.rest().v0().tip(&jormungandr.rest_uri());
    let block_counter_before_transaction = jormungandr.logger.get_created_blocks_counter();

    jcli.fragment_sender(&jormungandr)
        .send_many(transation_messages)
        .wait_until_all_processed(&Default::default())
        .map_err(NodeStuckError::InternalJcliError)?;

    let block_tip_after_transaction = jcli.rest().v0().tip(jormungandr.rest_uri());
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
    let jcli: JCli = Default::default();
    let account_state = jcli
        .rest()
        .v0()
        .account_stats(address, &jormungandr.rest_uri());

    if *account_state.value() != value {
        return Err(NodeStuckError::FundsNotTransfered {
            actual: *account_state.value(),
            expected: value,
            logs: jormungandr.logger.get_log_content(),
        });
    }
    Ok(())
}

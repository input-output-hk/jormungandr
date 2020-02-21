mod wait;

pub use wait::SyncWaitParams;

use crate::{
    node::NodeController,
    scenario::Controller,
    test::{ErrorKind, Result},
    wallet::Wallet,
};
use jormungandr_lib::{
    interfaces::FragmentStatus,
    testing::{Measurement, Thresholds},
    time::Duration as LibsDuration,
};
use std::{
    fmt, thread,
    time::{Duration, SystemTime},
};

pub fn wait_for_nodes_sync(sync_wait_params: &SyncWaitParams) {
    let wait_time = sync_wait_params.wait_time();
    std::thread::sleep(wait_time);
}

pub fn get_nodes_block_height_summary(nodes: Vec<&NodeController>) -> Vec<String> {
    nodes
        .iter()
        .map({
            |node| {
                return format!(
                    "node '{}' has block height: '{:?}'\n",
                    node.alias(),
                    node.stats().unwrap().last_block_height
                );
            }
        })
        .collect()
}

pub fn measure_and_log_sync_time(
    nodes: Vec<&NodeController>,
    sync_wait: Thresholds<Duration>,
    info: &str,
) {
    let now = SystemTime::now();
    while now.elapsed().unwrap() < sync_wait.max() {
        let block_heights: Vec<u32> = nodes
            .iter()
            .map(|node| {
                node.stats()
                    .unwrap()
                    .last_block_height
                    .unwrap()
                    .parse()
                    .unwrap()
            })
            .collect();
        let max_block_height = block_heights.iter().max().unwrap();
        if !block_heights.iter().any(|x| *x != *max_block_height) {
            log_measurement(Measurement::new(
                info.to_owned(),
                now.elapsed().unwrap(),
                sync_wait.clone(),
            ));
            return;
        }
    }

    // we know it fails, this method is used only for reporting
    assert_are_in_sync(SyncWaitParams::ZeroWait, nodes);
    log_measurement(Measurement::new(
        info.to_owned(),
        now.elapsed().unwrap(),
        sync_wait.clone(),
    ))
}

///temporary method for logging measurement which is currently printing content to console
fn log_measurement(measurement: Measurement<Duration>) {
    println!("{}", measurement);
}

pub fn assert_equals<A: fmt::Debug + PartialEq>(left: &A, right: &A, info: &str) -> Result<()> {
    if left != right {
        bail!(ErrorKind::AssertionFailed(format!(
            "{}. {:?} vs {:?}",
            info, left, right
        )))
    }
    Ok(())
}

pub fn assert_is_in_block(status: FragmentStatus, node: &NodeController) -> Result<()> {
    if !status.is_in_a_block() {
        bail!(ErrorKind::AssertionFailed(format!(
            "fragment status sent to node: {} is not in block :({:?}). logs: {}",
            node.alias(),
            status,
            node.log_content()
        )))
    }
    Ok(())
}

pub fn assert_are_in_sync(sync_wait: SyncWaitParams, nodes: Vec<&NodeController>) -> Result<()> {
    if nodes.len() < 2 {
        return Ok(());
    }

    wait_for_nodes_sync(&sync_wait);
    let duration: LibsDuration = sync_wait.wait_time().into();
    let first_node = nodes.iter().next().unwrap();

    let expected_block_hashes = first_node.all_blocks_hashes()?;
    let block_height = first_node.stats()?.last_block_height;

    for node in nodes.iter().skip(1) {
        let all_block_hashes = node.all_blocks_hashes()?;
        assert_equals(
            &expected_block_hashes,
            &all_block_hashes,
            &format!("nodes are out of sync (different block hashes) after sync grace period: ({}) . Left node: alias: {}, content: {}, Right node: alias: {}, content: {}",
                duration,
                first_node.alias(),
                first_node.log_content(),
                node.alias(),
                node.log_content()),
        )?;
        assert_equals(
            &block_height,
            &node.stats()?.last_block_height,
            &format!("nodes are out of sync (different block height) after sync grace period: ({}) . Left node: alias: {}, content: {}, Right node: alias: {}, content: {}",
                duration,
                first_node.alias(),
                first_node.log_content(),
                node.alias(),
                node.log_content()
                ),
        )?;
    }
    Ok(())
}

pub fn keep_sending_transaction_dispite_error(
    n: u32,
    controller: &mut Controller,
    mut wallet1: &mut Wallet,
    wallet2: &Wallet,
    node: &NodeController,
) -> Result<()> {
    for _ in 0..n {
        let check = controller.wallet_send_to(&mut wallet1, &wallet2, &node, 1_000.into());
        if let Err(err) = check {
            println!("ignoring error : {:?}", err);
        }
        thread::sleep(Duration::from_secs(1));
    }
    Ok(())
}

pub fn sending_transactions_to_node_sequentially(
    n: u32,
    controller: &mut Controller,
    mut wallet1: &mut Wallet,
    mut wallet2: &mut Wallet,
    node: &NodeController,
) -> Result<()> {
    for _ in 0..n {
        let check = controller.wallet_send_to(&mut wallet1, &wallet2, &node, 1_000.into())?;
        let status = node.wait_fragment(Duration::from_secs(2), check)?;
        assert_is_in_block(status, &node)?;
        let check = controller.wallet_send_to(&mut wallet2, &wallet1, &node, 1_000.into())?;
        let status = node.wait_fragment(Duration::from_secs(2), check)?;
        assert_is_in_block(status, &node)?;
    }
    Ok(())
}

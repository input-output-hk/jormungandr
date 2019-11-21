use crate::{
    node::NodeController,
    scenario::Controller,
    test::{ErrorKind, Result},
    wallet::Wallet,
};
use jormungandr_lib::interfaces::FragmentStatus;
use std::{fmt, thread, time::Duration};

pub fn assert_equals<A: fmt::Debug + PartialEq>(left: &A, right: &A, info: &str) -> Result<()> {
    if left != right {
        bail!(ErrorKind::AssertionFailed(format!(
            "{}. {:?} vs {:?}",
            info, left, right
        )))
    }
    Ok(())
}

pub fn assert_are_in_sync(nodes: Vec<&NodeController>) -> Result<()> {
    if nodes.len() < 2 {
        return Ok(());
    }

    let first_node = nodes.iter().next().unwrap();

    let expected_block_hashes = first_node.all_blocks_hashes()?;
    let block_height = first_node.stats()?.last_block_height;

    for node in nodes.iter().skip(1) {
        let all_block_hashes = node.all_blocks_hashes()?;
        assert_equals(
            &expected_block_hashes,
            &all_block_hashes,
            "nodes are out of sync (different bock hashes)",
        )?;
        assert_equals(
            &block_height,
            &node.stats()?.last_block_height,
            "nodes are out of sync (different bock height)",
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
    wallet2: &Wallet,
    node: &NodeController,
) -> Result<()> {
    for _ in 0..n {
        let check = controller
            .wallet_send_to(&mut wallet1, &wallet2, &node, 1_000.into())
            .unwrap();

        let status = node.wait_fragment(Duration::from_secs(2), check);

        if let Ok(status) = status {
            if status.is_in_a_block() {
                wallet1.confirm_transaction();
            } else {
                bail!(ErrorKind::TransactionNotInBlock(
                    node.alias().to_string(),
                    status
                ));
            }
        }
    }
    Ok(())
}

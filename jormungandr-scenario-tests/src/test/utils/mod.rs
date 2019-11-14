use crate::{node::NodeController, scenario::Controller, wallet::Wallet};
use std::{thread, time::Duration};

pub fn assert_are_in_sync(nodes: Vec<&NodeController>) {
    if nodes.len() < 2 {
        return;
    }

    let first_node = nodes.iter().next().unwrap();

    let expected_block_hashes = first_node.all_blocks_hashes().unwrap();
    let block_height = first_node.stats().unwrap().last_block_height;

    for node in nodes.iter().skip(1) {
        assert_eq!(
            expected_block_hashes,
            node.all_blocks_hashes().unwrap(),
            "nodes are out of sync (different bock hashes)"
        );
        assert_eq!(
            block_height,
            node.stats().unwrap().last_block_height,
            "nodes are out of sync (different bock height)"
        );
    }
}

pub fn keep_sending_transaction_to_node_until_error(
    n: u32,
    controller: &mut Controller,
    mut wallet1: &mut Wallet,
    wallet2: &Wallet,
    node: &NodeController,
) {
    for _ in 0..n {
        let check = controller
            .wallet_send_to(&mut wallet1, &wallet2, &node, 1_000.into())
            .unwrap();

        let status = node.wait_fragment(Duration::from_secs(2), check);

        if let Ok(status) = status {
            if status.is_in_a_block() {
                wallet1.confirm_transaction();
            } else {
                break;
            }
        } else {
            break;
        }
    }
}

pub fn keep_sending_transaction_dispite_error(
    n: u32,
    controller: &mut Controller,
    mut wallet1: &mut Wallet,
    wallet2: &Wallet,
    node: &NodeController,
) {
    for _ in 0..n {
        let check = controller.wallet_send_to(&mut wallet1, &wallet2, &node, 1_000.into());
        if let Err(err) = check {
            println!("{:?}", err);
        }
        thread::sleep(Duration::from_secs(1));
    }
}

pub fn sending_transactions_to_node_sequentially(
    n: u32,
    controller: &mut Controller,
    mut wallet1: &mut Wallet,
    wallet2: &Wallet,
    node: &NodeController,
) {
    for _ in 0..n {
        let check = controller
            .wallet_send_to(&mut wallet1, &wallet2, &node, 1_000.into())
            .unwrap();

        let status = node.wait_fragment(Duration::from_secs(2), check);

        if let Ok(status) = status {
            if status.is_in_a_block() {
                wallet1.confirm_transaction();
            } else {
                break;
            }
        } else {
            break;
        }
    }
}

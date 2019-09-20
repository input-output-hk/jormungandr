use crate::{node::NodeController, scenario::Controller, wallet::Wallet, Context};
use rand::RngCore;
use std::time::Duration;

pub fn keep_sending_transaction_to_node_until_error(
    controller: &mut Controller,
    mut wallet1: &mut Wallet,
    wallet2: &Wallet,
    node: &NodeController,
) {
    loop {
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

pub struct ArbitraryUSIZE;

impl ArbitraryUSIZE {
    pub fn prepare<RNG>(context: &mut Context<RNG>) -> usize
    where
        RNG: RngCore,
    {
        context.rng_mut().next_u32() as usize
    }
}

pub struct ArbitraryNodeController;

impl ArbitraryNodeController {
    pub fn prepare<RNG>(
        node_controllers: Vec<NodeController>,
        context: &mut Context<RNG>,
    ) -> NodeController
    where
        RNG: RngCore,
    {
        let random_index = ArbitraryUSIZE::prepare(context) % node_controllers.len();
        node_controllers.get(random_index).cloned().unwrap()
    }
}

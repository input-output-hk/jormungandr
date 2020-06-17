use super::InteractiveCommandError;
use crate::{legacy::LegacyNodeController, test::Result};
use crate::{node::NodeController, scenario::Controller, style};
use jormungandr_testing_utils::wallet::Wallet;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Send {
    /// Sends transaction
    Tx(SendTransaction),
}

impl Send {
    pub fn exec(
        &self,
        controller: &mut Controller,
        nodes: &mut Vec<NodeController>,
        legacy_nodes: &mut Vec<LegacyNodeController>,
        wallets: &mut Vec<Wallet>,
    ) -> Result<()> {
        match self {
            Send::Tx(transaction) => transaction.exec(controller, nodes, legacy_nodes, wallets),
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct SendTransaction {
    #[structopt(short = "f", long = "from")]
    pub from: String,
    #[structopt(short = "t", long = "to")]
    pub to: String,
    #[structopt(short = "v", long = "via")]
    pub via: String,
    #[structopt(short = "a", long = "ada")]
    pub ada: Option<u64>,
}

impl SendTransaction {
    pub fn exec(
        &self,
        controller: &mut Controller,
        nodes: &mut Vec<NodeController>,
        legacy_nodes: &mut Vec<LegacyNodeController>,
        wallets: &mut Vec<Wallet>,
    ) -> Result<()> {
        let from_address = controller.wallet(&self.from)?.address();
        let to_address = controller.wallet(&self.to)?.address();

        let to = wallets
            .iter()
            .cloned()
            .find(|x| x.address() == to_address)
            .expect(&format!("cannot find wallet with alias: {}", self.to));
        let mut from = wallets
            .iter_mut()
            .find(|x| x.address() == from_address)
            .expect(&format!("cannot find wallet with alias: {}", self.from));

        let node = nodes.iter().find(|x| *x.alias() == self.via);
        let legacy_node = legacy_nodes.iter().find(|x| *x.alias() == self.via);

        if let Some(node) = node {
            let mem_pool_check = controller.fragment_sender().send_transaction(
                &mut from,
                &to,
                node,
                self.ada.unwrap_or(100).into(),
            )?;
            println!(
                "{}",
                style::info.apply_to(format!(
                    "fragment '{}' successfully sent",
                    mem_pool_check.fragment_id()
                ))
            );
            return Ok(());
        } else if let Some(legacy_node) = legacy_node {
            let mem_pool_check = controller.fragment_sender().send_transaction(
                &mut from,
                &to,
                legacy_node,
                self.ada.unwrap_or(100).into(),
            )?;
            println!(
                "{}",
                style::info.apply_to(format!(
                    "fragment '{}' successfully sent",
                    mem_pool_check.fragment_id()
                ))
            );
            return Ok(());
        }

        Err(InteractiveCommandError::NodeAliasNotFound(self.via.clone()))?
    }
}

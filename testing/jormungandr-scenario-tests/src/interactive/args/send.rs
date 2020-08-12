use super::UserInteractionController;
use crate::{style, test::Result};
use jortestkit::prelude::InteractiveCommandError;
use structopt::StructOpt;
#[derive(StructOpt, Debug)]
pub enum Send {
    /// Sends transaction
    Tx(SendTransaction),
}

impl Send {
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<()> {
        match self {
            Send::Tx(transaction) => transaction.exec(controller),
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
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<()> {
        let node = controller
            .nodes()
            .iter()
            .cloned()
            .find(|x| *x.alias() == self.via);
        let legacy_node = controller
            .legacy_nodes()
            .iter()
            .cloned()
            .find(|x| *x.alias() == self.via);

        if let Some(node) = node {
            let mem_pool_check = controller.send_transaction(
                &self.from,
                &self.to,
                &node,
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
            let mem_pool_check = controller.send_transaction(
                &self.from,
                &self.to,
                &legacy_node,
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

        Err(InteractiveCommandError::UserError(format!(
            "alias not found {}",
            self.via.clone()
        )))?
    }
}

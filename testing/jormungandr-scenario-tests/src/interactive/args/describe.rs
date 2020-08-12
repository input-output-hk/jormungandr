use crate::{style, test::Result};

use super::UserInteractionController;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Describe {
    /// Prints available wallets with aliases
    /// that can be used
    Wallets(DescribeWallets),
    /// Prints available node with aliases
    /// that can be used
    Nodes(DescribeNodes),
    /// Prints trusted peer info
    Topology,
    /// Prints everything
    All(DescribeAll),
}

impl Describe {
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<()> {
        match self {
            Describe::Wallets(wallets) => wallets.exec(controller),
            Describe::Nodes(desc_nodes) => desc_nodes.exec(controller),
            Describe::All(all) => all.exec(controller),
            Describe::Topology => {
                println!(
                    "{}",
                    style::info.apply_to("Legend: '->' means trust direction".to_owned())
                );
                for (alias, node) in controller.controller().topology().clone().into_iter() {
                    println!(
                        "\t{} -> {:?}",
                        alias,
                        node.trusted_peers().collect::<Vec<&String>>()
                    )
                }
                Ok(())
            }
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct DescribeWallets {
    #[structopt(short = "a", long = "alias")]
    pub alias: Option<String>,
}

impl DescribeWallets {
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<()> {
        println!("Wallets:");
        for (alias, wallet) in controller.controller().wallets() {
            println!(
                "\t{}: address: {}, delegated to: {:?}",
                alias,
                wallet.address(),
                wallet.template().delegate()
            );
        }
        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct DescribeNodes {
    #[structopt(short = "a", long = "alias")]
    pub alias: Option<String>,
}

impl DescribeNodes {
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<()> {
        println!("Nodes:");
        for (alias, node) in controller.controller().nodes() {
            println!("\t{}: rest api: {}", alias, node.config().rest.listen);
        }
        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct DescribeAll {
    #[structopt(short = "a", long = "alias")]
    pub alias: Option<String>,
}

impl DescribeAll {
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<()> {
        let describe_wallets = DescribeWallets { alias: None };
        describe_wallets.exec(controller)?;
        let describe_nodes = DescribeNodes { alias: None };
        describe_nodes.exec(controller)
    }
}

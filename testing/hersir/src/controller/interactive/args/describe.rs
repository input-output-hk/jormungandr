use crate::{
    controller::{Error, UserInteractionController},
    style,
};
use chain_impl_mockchain::certificate::VotePlan;
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
    Topology(DescribeTopology),
    /// Prints everything
    All(DescribeAll),
    /// Prints Votes Plan
    VotePlan(DescribeVotePlans),
}

impl Describe {
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<(), Error> {
        match self {
            Describe::Wallets(wallets) => wallets.exec(controller),
            Describe::Nodes(desc_nodes) => desc_nodes.exec(controller),
            Describe::All(all) => all.exec(controller),
            Describe::Topology(topology) => topology.exec(controller),
            Describe::VotePlan(vote_plans) => vote_plans.exec(controller),
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct DescribeTopology {
    #[structopt(short = "a", long = "alias")]
    pub alias: Option<String>,
}

impl DescribeTopology {
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<(), Error> {
        println!(
            "{}",
            style::info.apply_to("Legend: '->' means trust direction".to_owned())
        );
        for (alias, node) in controller.controller().settings().nodes.iter() {
            println!(
                "\t{} -> {:?}",
                alias,
                node.node_topology
                    .trusted_peers
                    .iter()
                    .collect::<Vec<&String>>()
            )
        }
        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct DescribeWallets {
    #[structopt(short = "a", long = "alias")]
    pub alias: Option<String>,
}

impl DescribeWallets {
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<(), Error> {
        println!("Wallets:");
        for (alias, wallet) in controller.controller().defined_wallets() {
            println!(
                "\t{}: address: {}, initial_funds: {}, delegated to: {:?}",
                alias,
                wallet.address()?,
                wallet.template().value(),
                wallet.template().delegate()
            );
        }
        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct DescribeVotePlans {
    #[structopt(short = "a", long = "alias")]
    pub alias: Option<String>,
}

impl DescribeVotePlans {
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<(), Error> {
        println!("Vote Plans:");
        for vote_plan in controller.controller().defined_vote_plans() {
            let chain_vote_plan: VotePlan = vote_plan.clone().into();
            println!(
                "\t{}:\n\t - owner: {}\n\t - id: {}\n\t - start: {}\n\t - tally: {}\n\t - end: {}\n",
                vote_plan.alias(),
                vote_plan.owner(),
                vote_plan.id(),
                chain_vote_plan.vote_start(),
                chain_vote_plan.committee_start(),
                chain_vote_plan.committee_end()
            );
            println!("\tProposals");

            for proposal in vote_plan.proposals() {
                println!("\t\t{}", hex::encode(proposal.id()))
            }
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
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<(), Error> {
        println!("Nodes:");
        for (alias, node) in controller.controller().defined_nodes() {
            println!("\t{}: rest api: {}", alias, node.config.rest.listen);
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
    pub fn exec(&self, controller: &mut UserInteractionController) -> Result<(), Error> {
        let describe_wallets = DescribeWallets { alias: None };
        describe_wallets.exec(controller)?;
        let describe_nodes = DescribeNodes { alias: None };
        describe_nodes.exec(controller)
    }
}

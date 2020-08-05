use super::do_for_all_alias;
use crate::node::NodeController;
use crate::{legacy::LegacyNodeController, test::Result};
use jormungandr_testing_utils::testing::node::JormungandrLogger;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Show {
    /// Prints which nodes are upp
    Status(ShowStatus),
    /// Prints fragments counts
    FragmentCount(ShowFragmentCount),
    /// Prints received fragment list
    Fragments(ShowFragments),
    /// Prints block height
    BlockHeight(ShowBlockHeight),
    /// Prints peers stats
    PeerStats(ShowPeerStats),
    /// Prints stats
    Stats(ShowNodeStats),
    /// Prints logs, can filter logs to print
    /// only errors or filter by custom string  
    Logs(ShowLogs),
}

#[derive(StructOpt, Debug)]
pub struct ShowStatus {
    #[structopt(short = "a", long = "alias")]
    pub alias: Option<String>,
}

#[derive(StructOpt, Debug)]
pub struct ShowNodeStats {
    #[structopt(short = "a", long = "alias")]
    pub alias: Option<String>,
}

#[derive(StructOpt, Debug)]
pub struct ShowLogs {
    #[structopt(short = "a", long = "alias")]
    pub alias: Option<String>,

    #[structopt(short = "e", long = "only-errors")]
    pub only_errors: bool,

    #[structopt(short = "c", long = "contains")]
    pub contains: Option<String>,

    #[structopt(short = "t", long = "tail")]
    pub tail: Option<usize>,
}

#[derive(StructOpt, Debug)]
pub struct ShowFragmentCount {
    #[structopt(short = "a", long = "alias")]
    pub alias: Option<String>,
}

#[derive(StructOpt, Debug)]
pub struct ShowFragments {
    #[structopt(short = "a", long = "alias")]
    pub alias: Option<String>,
}

#[derive(StructOpt, Debug)]
pub struct ShowBlockHeight {
    #[structopt(short = "a", long = "alias")]
    pub alias: Option<String>,
}

#[derive(StructOpt, Debug)]
pub struct ShowPeerStats {
    #[structopt(short = "a", long = "alias")]
    pub alias: Option<String>,
}

impl ShowStatus {
    pub fn exec(
        &self,
        nodes: &mut Vec<NodeController>,
        legacy_nodes: &mut Vec<LegacyNodeController>,
    ) -> Result<()> {
        do_for_all_alias(
            &self.alias,
            nodes,
            legacy_nodes,
            |node| println!("{} is up", node.alias()),
            |node| println!("{} is up", node.alias()),
        )
    }
}

impl ShowFragmentCount {
    pub fn exec(
        &self,
        nodes: &mut Vec<NodeController>,
        legacy_nodes: &mut Vec<LegacyNodeController>,
    ) -> Result<()> {
        do_for_all_alias(
            &self.alias,
            nodes,
            legacy_nodes,
            |node| {
                println!(
                    "{}: {:#?}",
                    node.alias(),
                    node.fragment_logs().unwrap().len()
                )
            },
            |node| println!("{}: {}", node.alias(), node.fragment_logs().unwrap().len()),
        )
    }
}

impl ShowFragments {
    pub fn exec(
        &self,
        nodes: &mut Vec<NodeController>,
        legacy_nodes: &mut Vec<LegacyNodeController>,
    ) -> Result<()> {
        do_for_all_alias(
            &self.alias,
            nodes,
            legacy_nodes,
            |node| println!("{}: {:#?}", node.alias(), node.fragment_logs().unwrap()),
            |node| {
                println!(
                    "{}: {:#?}",
                    node.alias(),
                    node.fragment_logs().unwrap().len()
                )
            },
        )
    }
}

impl ShowBlockHeight {
    pub fn exec(
        &self,
        nodes: &mut Vec<NodeController>,
        legacy_nodes: &mut Vec<LegacyNodeController>,
    ) -> Result<()> {
        do_for_all_alias(
            &self.alias,
            nodes,
            legacy_nodes,
            |node| {
                println!(
                    "{}: {:?}",
                    node.alias(),
                    node.stats().unwrap().stats.unwrap().last_block_height
                )
            },
            |node| {
                println!(
                    "{}: {:?}",
                    node.alias(),
                    node.stats().unwrap()["stats"]["last_block_height"].to_owned()
                )
            },
        )
    }
}

impl ShowPeerStats {
    pub fn exec(
        &self,
        nodes: &mut Vec<NodeController>,
        legacy_nodes: &mut Vec<LegacyNodeController>,
    ) -> Result<()> {
        do_for_all_alias(
            &self.alias,
            nodes,
            legacy_nodes,
            |node| println!("{} is up", node.alias()),
            |node| println!("{} is up", node.alias()),
        )
    }
}

impl ShowNodeStats {
    pub fn exec(
        &self,
        nodes: &mut Vec<NodeController>,
        legacy_nodes: &mut Vec<LegacyNodeController>,
    ) -> Result<()> {
        do_for_all_alias(
            &self.alias,
            nodes,
            legacy_nodes,
            |node| println!("{}: {:#?}", node.alias(), node.stats()),
            |node| println!("{}: {:#?}", node.alias(), node.stats()),
        )
    }
}

fn show_logs_for(
    only_errors: bool,
    contains: &Option<String>,
    alias: &str,
    tail: Option<usize>,
    logger: JormungandrLogger,
) {
    let logs: Vec<String> = {
        if only_errors {
            logger.get_lines_with_error().collect()
        } else if let Some(contains) = &contains {
            logger
                .get_lines_from_log()
                .filter(|x| x.contains(contains.as_str()))
                .collect()
        } else if let Some(tail) = tail {
            logger
                .get_lines_from_log()
                .collect::<Vec<String>>()
                .iter()
                .cloned()
                .rev()
                .take(tail)
                .collect()
        } else {
            logger.get_lines_from_log().collect()
        }
    };

    println!("{}:", alias);

    for log in logs {
        println!("\t{}", log);
    }
}

impl ShowLogs {
    pub fn exec(
        &self,
        nodes: &mut Vec<NodeController>,
        legacy_nodes: &mut Vec<LegacyNodeController>,
    ) -> Result<()> {
        do_for_all_alias(
            &self.alias,
            nodes,
            legacy_nodes,
            |node| {
                show_logs_for(
                    self.only_errors,
                    &self.contains,
                    node.alias(),
                    self.tail,
                    node.logger(),
                )
            },
            |node| {
                show_logs_for(
                    self.only_errors,
                    &self.contains,
                    node.alias(),
                    self.tail,
                    node.logger(),
                )
            },
        )
    }
}

impl Show {
    pub fn exec(
        &self,
        nodes: &mut Vec<NodeController>,
        legacy_nodes: &mut Vec<LegacyNodeController>,
    ) -> Result<()> {
        match self {
            Show::Status(status) => status.exec(nodes, legacy_nodes),
            Show::Stats(stats) => stats.exec(nodes, legacy_nodes),
            Show::FragmentCount(fragment_counts) => fragment_counts.exec(nodes, legacy_nodes),
            Show::Fragments(fragments) => fragments.exec(nodes, legacy_nodes),
            Show::BlockHeight(block_height) => block_height.exec(nodes, legacy_nodes),
            Show::PeerStats(peer_stats) => peer_stats.exec(nodes, legacy_nodes),
            Show::Logs(logs) => logs.exec(nodes, legacy_nodes),
        }
    }
}

use crate::controller::{do_for_all_alias, UserInteractionController};
use jormungandr_automation::jormungandr::{
    BackwardCompatibleRest, FragmentNode, JormungandrLogger, LogLevel,
};
use structopt::StructOpt;
use yaml_rust::Yaml;

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
    /// Active Vote Plans
    VotePlans(ActiveVotePlans),
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
pub struct ActiveVotePlans {
    #[structopt(short = "a", long = "alias")]
    pub alias: Option<String>,
}

impl ActiveVotePlans {
    pub fn exec(&self, controller: &mut UserInteractionController) {
        do_for_all_alias(
            &self.alias,
            controller.nodes(),
            controller.legacy_nodes(),
            |node| println!("{}: {:#?}", node.alias(), node.rest().vote_plan_statuses()),
            |node| println!("{}: {:#?}", node.alias(), node.rest().vote_plan_statuses()),
        )
    }
}

#[derive(StructOpt, Debug)]
pub struct ShowPeerStats {
    #[structopt(short = "a", long = "alias")]
    pub alias: Option<String>,
}

#[derive(StructOpt, Debug)]
pub struct ShowStatus {
    #[structopt(short = "a", long = "alias")]
    pub alias: Option<String>,
}

impl ShowStatus {
    pub fn exec(&self, controller: &mut UserInteractionController) {
        do_for_all_alias(
            &self.alias,
            controller.nodes(),
            controller.legacy_nodes(),
            |node| println!("{} is up", node.alias()),
            |node| println!("{} is up", &node.alias()),
        )
    }
}

impl ShowFragmentCount {
    pub fn exec(&self, controller: &mut UserInteractionController) {
        do_for_all_alias(
            &self.alias,
            controller.nodes(),
            controller.legacy_nodes(),
            |node| {
                println!(
                    "{}: {:#?}",
                    node.alias(),
                    node.fragment_logs().unwrap().len()
                )
            },
            |node| {
                println!(
                    "{}: {}",
                    node.alias(),
                    node.rest().fragment_logs().unwrap().len()
                )
            },
        )
    }
}

impl ShowFragments {
    pub fn exec(&self, controller: &mut UserInteractionController) {
        do_for_all_alias(
            &self.alias,
            controller.nodes(),
            controller.legacy_nodes(),
            |node| println!("{}: {:#?}", node.alias(), node.fragment_logs().unwrap()),
            |node| {
                println!(
                    "{}: {:#?}",
                    node.alias(),
                    node.rest().fragment_logs().unwrap().len()
                )
            },
        )
    }
}

impl ShowBlockHeight {
    pub fn exec(&self, controller: &mut UserInteractionController) {
        do_for_all_alias(
            &self.alias,
            controller.nodes(),
            controller.legacy_nodes(),
            |node| {
                println!(
                    "{}: {:?}",
                    node.alias(),
                    node.rest()
                        .stats()
                        .unwrap()
                        .stats
                        .unwrap()
                        .last_block_height
                )
            },
            |node| {
                println!("{}: {:?}", node.alias(), {
                    let stats = Yaml::from_str(
                        &BackwardCompatibleRest::new(
                            node.rest_address().to_string(),
                            Default::default(),
                        )
                        .stats()
                        .unwrap(),
                    );
                    stats["stats"]["last_block_height"].to_owned()
                })
            },
        )
    }
}

impl ShowPeerStats {
    pub fn exec(&self, controller: &mut UserInteractionController) {
        do_for_all_alias(
            &self.alias,
            controller.nodes(),
            controller.legacy_nodes(),
            |node| println!("{} is up", node.alias()),
            |node| println!("{} is up", node.alias()),
        )
    }
}

impl ShowNodeStats {
    pub fn exec(&self, controller: &mut UserInteractionController) {
        do_for_all_alias(
            &self.alias,
            controller.nodes(),
            controller.legacy_nodes(),
            |node| println!("{}: {:#?}", node.alias(), node.rest().stats()),
            |node| println!("{}: {:#?}", node.alias(), node.rest().stats()),
        )
    }
}

fn show_logs_for(
    only_errors: bool,
    contains: &Option<String>,
    alias: &str,
    tail: Option<usize>,
    logger: &JormungandrLogger,
) {
    let logs: Vec<String> = {
        if only_errors {
            logger
                .get_log_lines_with_level(LogLevel::ERROR)
                .map(|x| x.to_string())
                .collect()
        } else if let Some(contains) = &contains {
            logger
                .get_lines()
                .into_iter()
                .filter(|x| x.message().contains(contains.as_str()))
                .map(|x| x.to_string())
                .collect()
        } else if let Some(tail) = tail {
            logger
                .get_lines_as_string()
                .into_iter()
                .rev()
                .take(tail)
                .collect()
        } else {
            logger.get_lines_as_string()
        }
    };

    println!("{}:", alias);

    for log in logs {
        println!("\t{}", log);
    }
}

impl ShowLogs {
    pub fn exec(&self, controller: &mut UserInteractionController) {
        do_for_all_alias(
            &self.alias,
            controller.nodes(),
            controller.legacy_nodes(),
            |node| {
                show_logs_for(
                    self.only_errors,
                    &self.contains,
                    &node.alias(),
                    self.tail,
                    &node.logger,
                )
            },
            |node| {
                show_logs_for(
                    self.only_errors,
                    &self.contains,
                    &node.alias(),
                    self.tail,
                    &node.logger,
                )
            },
        )
    }
}

impl Show {
    pub fn exec(&self, controller: &mut UserInteractionController) {
        match self {
            Show::Status(status) => status.exec(controller),
            Show::Stats(stats) => stats.exec(controller),
            Show::FragmentCount(fragment_counts) => fragment_counts.exec(controller),
            Show::Fragments(fragments) => fragments.exec(controller),
            Show::BlockHeight(block_height) => block_height.exec(controller),
            Show::PeerStats(peer_stats) => peer_stats.exec(controller),
            Show::Logs(logs) => logs.exec(controller),
            Show::VotePlans(active_vote_plan) => active_vote_plan.exec(controller),
        }
    }
}

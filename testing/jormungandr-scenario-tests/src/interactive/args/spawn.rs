use super::InteractiveCommandError;
use crate::{legacy::LegacyNodeController, test::Result};
use crate::{node::NodeController, scenario::Controller, style};
use jormungandr_testing_utils::testing::{
    network_builder::{LeadershipMode, PersistenceMode, SpawnParams},
    node::download_last_n_releases,
};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Spawn {
    Passive(SpawnPassiveNode),
    Leader(SpawnLeaderNode),
}

impl Spawn {
    pub fn exec(
        &self,
        controller: &mut Controller,
        nodes: &mut Vec<NodeController>,
        legacy_nodes: &mut Vec<LegacyNodeController>,
    ) -> Result<()> {
        match self {
            Spawn::Passive(spawn_passive) => spawn_passive.exec(controller, nodes, legacy_nodes),
            Spawn::Leader(spawn_leader) => spawn_leader.exec(controller, nodes, legacy_nodes),
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct SpawnPassiveNode {
    #[structopt(short = "s", long = "storage")]
    pub storage: bool,
    #[structopt(short = "l", long = "legacy")]
    pub legacy: Option<String>,
    #[structopt(short = "w", long = "wait")]
    pub wait: bool,
    #[structopt(short = "a", long = "alias")]
    pub alias: String,
}

impl SpawnPassiveNode {
    pub fn exec(
        &self,
        mut controller: &mut Controller,
        mut nodes: &mut Vec<NodeController>,
        mut legacy_nodes: &mut Vec<LegacyNodeController>,
    ) -> Result<()> {
        spawn_node(
            &mut controller,
            LeadershipMode::Passive,
            self.storage,
            &self.alias,
            self.legacy.clone(),
            self.wait,
            &mut nodes,
            &mut legacy_nodes,
        )
    }
}

#[derive(StructOpt, Debug)]
pub struct SpawnLeaderNode {
    #[structopt(short = "s", long = "storage")]
    pub storage: bool,
    #[structopt(short = "l", long = "legacy")]
    pub legacy: Option<String>,
    #[structopt(short = "w", long = "wait")]
    pub wait: bool,
    #[structopt(short = "a", long = "alias")]
    pub alias: String,
}

fn spawn_node(
    controller: &mut Controller,
    leadership_mode: LeadershipMode,
    storage: bool,
    alias: &str,
    legacy: Option<String>,
    wait: bool,
    nodes: &mut Vec<NodeController>,
    legacy_nodes: &mut Vec<LegacyNodeController>,
) -> Result<()> {
    let persistence_mode = {
        if storage {
            PersistenceMode::Persistent
        } else {
            PersistenceMode::InMemory
        }
    };

    let mut spawn_params = SpawnParams::new(alias);
    spawn_params
        .persistence_mode(persistence_mode)
        .leadership_mode(leadership_mode);

    if let Some(version) = legacy {
        let releases = download_last_n_releases(5);
        let legacy_release = releases
            .iter()
            .find(|x| x.version().eq_ignore_ascii_case(&version))
            .ok_or(InteractiveCommandError::VersionNotFound(
                version.to_string(),
            ))?;

        let node = controller.spawn_legacy_node(
            &mut spawn_params,
            &legacy_release.version().parse().unwrap(),
        )?;
        println!(
            "{}",
            style::info.apply_to(format!("node '{}' spawned", alias))
        );

        if wait {
            println!(
                "{}",
                style::info.apply_to("waiting for bootstap...".to_string())
            );
            node.wait_for_bootstrap()?;
            println!(
                "{}",
                style::info.apply_to("node bootstrapped successfully.".to_string())
            );
        }

        legacy_nodes.push(node);
        return Ok(());
    }

    let node = controller.spawn_node_custom(&mut spawn_params)?;
    println!(
        "{}",
        style::info.apply_to(format!("node '{}' spawned", alias))
    );

    if wait {
        println!(
            "{}",
            style::info.apply_to("waiting for bootstap...".to_string())
        );
        node.wait_for_bootstrap()?;
        println!(
            "{}",
            style::info.apply_to("node bootstrapped successfully.".to_string())
        );
    }

    nodes.push(node);
    Ok(())
}

impl SpawnLeaderNode {
    pub fn exec(
        &self,
        mut controller: &mut Controller,
        mut nodes: &mut Vec<NodeController>,
        mut legacy_nodes: &mut Vec<LegacyNodeController>,
    ) -> Result<()> {
        spawn_node(
            &mut controller,
            LeadershipMode::Leader,
            self.storage,
            &self.alias,
            self.legacy.clone(),
            self.wait,
            &mut nodes,
            &mut legacy_nodes,
        )
    }
}

use crate::{legacy::LegacyNodeController, test::Result};
use crate::{node::NodeController, scenario::Controller};
use jormungandr_testing_utils::wallet::Wallet;
use structopt::{clap::AppSettings, StructOpt};

use thiserror::Error;

mod describe;
mod send;
mod show;
mod spawn;

#[derive(Error, Debug)]
pub enum InteractiveCommandError {
    #[error("cannot spawn not with version '{0}', looks like it's incorrect one")]
    VersionNotFound(String),
    #[error("cannot find node with alias(0). Please run 'describe' command ")]
    NodeAliasNotFound(String),
}

#[derive(StructOpt, Debug)]
#[structopt(setting = AppSettings::NoBinaryName)]
pub enum InteractiveCommand {
    /// Prints nodes related data, like stats,fragments etc.
    Show(show::Show),
    /// Spawn leader or passive node (also legacy)
    Spawn(spawn::Spawn),
    /// Exit interactive mode
    Exit,
    /// Prints wallets, nodes which can be used. Draw topology
    Describe(describe::Describe),
    /// send fragments
    Send(send::Send),
}
impl InteractiveCommand {
    pub fn exec(
        &self,
        controller: &mut Controller,
        nodes: &mut Vec<NodeController>,
        legacy_nodes: &mut Vec<LegacyNodeController>,
        wallets: &mut Vec<Wallet>,
    ) -> Result<()> {
        match self {
            InteractiveCommand::Show(show) => show.exec(nodes, legacy_nodes),
            InteractiveCommand::Spawn(spawn) => spawn.exec(controller, nodes, legacy_nodes),
            InteractiveCommand::Exit => Ok(()),
            InteractiveCommand::Describe(describe) => describe.exec(controller),
            InteractiveCommand::Send(send) => send.exec(controller, nodes, legacy_nodes, wallets),
        }
    }
}

fn do_for_all_alias<F: Fn(&NodeController), G: Fn(&LegacyNodeController)>(
    alias: &Option<String>,
    nodes: &mut Vec<NodeController>,
    legacy_nodes: &mut Vec<LegacyNodeController>,
    f: F,
    g: G,
) -> Result<()> {
    if let Some(alias) = alias {
        if let Some(node) = nodes.iter().find(|x| *x.alias() == *alias) {
            f(node);
        }
        if let Some(node) = legacy_nodes.iter().find(|x| *x.alias() == *alias) {
            g(node)
        }
        return Ok(());
    }

    for node in nodes.iter() {
        f(node);
    }
    for node in legacy_nodes.iter() {
        g(node);
    }
    Ok(())
}

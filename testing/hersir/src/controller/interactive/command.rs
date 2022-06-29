use super::args::{describe, explorer, send, show, spawn};
use structopt::{clap::AppSettings, StructOpt};

#[derive(StructOpt, Debug)]
#[structopt(setting = AppSettings::NoBinaryName)]
pub enum InteractiveCommand {
    /// Prints nodes related data, like stats,fragments etc.
    Show(show::Show),
    /// Spawn leader or passive node (also legacy)
    Spawn(spawn::Spawn),
    /// Sends Explorer queries
    Explorer(explorer::Explorer),
    /// Exit interactive mode
    Exit,
    /// Prints wallets, nodes which can be used. Draw topology
    Describe(describe::Describe),
    /// send fragments
    Send(send::Send),
}

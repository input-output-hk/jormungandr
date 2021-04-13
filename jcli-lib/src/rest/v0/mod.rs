mod account;
mod block;
mod diagnostic;
mod leaders;
mod message;
mod network;
mod node;
mod rewards;
mod settings;
mod shutdown;
mod stake;
mod stake_pool;
mod stake_pools;
mod tip;
mod utxo;
mod vote;

use crate::jcli_lib::rest::Error;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum V0 {
    /// Account operations
    Account(account::Account),
    /// Block operations
    Block(block::Block),
    /// Node leaders operations
    Leaders(leaders::Leaders),
    /// Message sending
    Message(message::Message),
    /// Network information
    Network(network::Network),
    /// Node information
    Node(node::Node),
    /// Node settings
    Settings(settings::Settings),
    /// Stake information
    Stake(stake::Stake),
    /// Stake pool operations
    StakePool(stake_pool::StakePool),
    /// Stake pools operations
    StakePools(stake_pools::StakePools),
    /// Shutdown node
    Shutdown(shutdown::Shutdown),
    /// Blockchain tip information
    Tip(tip::Tip),
    /// UTXO information
    Utxo(utxo::Utxo),
    /// System diagnostic information
    Diagnostic(diagnostic::Diagnostic),
    /// Rewards information
    Rewards(rewards::Rewards),
    /// Vote related operations
    Vote(vote::Vote),
}

impl V0 {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            V0::Account(account) => account.exec(),
            V0::Block(block) => block.exec(),
            V0::Leaders(leaders) => leaders.exec(),
            V0::Message(message) => message.exec(),
            V0::Network(network) => network.exec(),
            V0::Node(node) => node.exec(),
            V0::Settings(settings) => settings.exec(),
            V0::Stake(stake) => stake.exec(),
            V0::StakePool(stake_pool) => stake_pool.exec(),
            V0::StakePools(stake_pools) => stake_pools.exec(),
            V0::Shutdown(shutdown) => shutdown.exec(),
            V0::Tip(tip) => tip.exec(),
            V0::Utxo(utxo) => utxo.exec(),
            V0::Diagnostic(diagnostic) => diagnostic.exec(),
            V0::Rewards(rewards) => rewards.exec(),
            V0::Vote(vote) => vote.exec(),
        }
    }
}

#[macro_use(error_chain, bail)]
extern crate error_chain;
#[macro_use(lazy_static)]
extern crate lazy_static;

pub mod node;
mod programs;
#[macro_use]
pub mod scenario;
pub mod example_scenarios;
mod slog;
pub mod style;
pub mod test;
pub use self::node::{
    LeadershipMode, MemPoolCheck, Node, NodeBlock0, NodeController, PersistenceMode, Status,
};
pub use self::programs::prepare_command;
pub use self::scenario::{
    repository::{parse_tag_from_str, ScenarioResult, ScenariosRepository, Tag},
    Context, Controller, NodeAlias, Seed, WalletAlias, WalletType,
};
pub use self::slog::{Error as SlogCodecError, SlogCodec};
pub use jormungandr_testing_utils::testing::network_builder::Wallet;

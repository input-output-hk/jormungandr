#[macro_use(error_chain, bail)]
extern crate error_chain;

pub mod node;
mod programs;
#[macro_use]
pub mod scenario;
mod slog;
mod wallet;

pub use self::node::Node;
pub use self::programs::prepare_command;
pub use self::scenario::{Context, NodeAlias, Seed, WalletAlias, WalletType};
pub use self::slog::{Error as SlogCodecError, SlogCodec};
pub use self::wallet::Wallet;

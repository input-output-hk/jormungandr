use crate::controller::InteractiveCommandError;
use jormungandr_automation::jormungandr::ExplorerError;
use jormungandr_automation::jormungandr::LegacyConfigConverterError;
use jormungandr_automation::jormungandr::StartupError;
use thiserror::Error;
use thor::FragmentSenderError;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Node(#[from] super::monitor::NodeError),

    #[error(transparent)]
    Wallet(#[from] thor::WalletError),

    #[error(transparent)]
    FsFixture(#[from] assert_fs::fixture::FixtureError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    Explorer(#[from] ExplorerError),

    #[error(transparent)]
    BlockFormatError(#[from] chain_core::mempack::ReadError),

    #[error("No node with alias {0}")]
    NodeNotFound(String),

    #[error("Wallet '{0}' was not found. Used before or never initialize")]
    WalletNotFound(String),

    #[error("StakePool '{0}' was not found. Used before or never initialize")]
    StakePoolNotFound(String),

    #[error("VotePlan '{0}' was not found. Used before or never initialize")]
    VotePlanNotFound(String),

    #[error(transparent)]
    Startup(#[from] StartupError),

    #[error("cannot spawn the node")]
    CannotSpawnNode(#[source] std::io::Error),

    #[error(transparent)]
    LegacyConfigConverter(#[from] LegacyConfigConverterError),

    #[error(transparent)]
    InteractiveCommand(#[from] InteractiveCommandError),

    #[error(transparent)]
    FragmentSender(#[from] FragmentSenderError),

    #[error(transparent)]
    Serialization(#[from] serde_yaml::Error),
}

pub mod comm;
pub mod features;
pub mod legacy;
pub mod network;
pub mod non_functional;
pub mod utils;

use jormungandr_lib::interfaces::FragmentStatus;
use jormungandr_testing_utils::testing::jormungandr::StartupError;

use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Interactive(#[from] jortestkit::console::InteractiveCommandError),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    Node(#[from] crate::node::Error),

    #[error(transparent)]
    Wallet(#[from] jormungandr_testing_utils::wallet::WalletError),

    #[error(transparent)]
    FragmentSender(#[from] jormungandr_testing_utils::testing::FragmentSenderError),

    #[error(transparent)]
    FragmentVerifier(#[from] jormungandr_testing_utils::testing::FragmentVerifierError),

    #[error(transparent)]
    VerificationFailed(#[from] jormungandr_testing_utils::testing::VerificationError),

    #[error(transparent)]
    MonitorResourcesError(#[from] jormungandr_testing_utils::testing::ConsumptionBenchmarkError),

    #[error(transparent)]
    ExplorerError(#[from] jormungandr_testing_utils::testing::node::ExplorerError),

    #[error(transparent)]
    Scenario(#[from] crate::scenario::Error),

    #[error("synchronization for nodes has failed. {info}. Timeout was: {} s", timeout.as_secs())]
    SyncTimeoutOccurred { info: String, timeout: Duration },

    #[error("assertion failed: {0}")]
    AssertionFailed(String),

    #[error("transaction should be 'In Block'. status: {status:?}, node: {node}")]
    TransactionNotInBlock {
        node: String,
        status: FragmentStatus,
    },
    #[error(transparent)]
    Rest(#[from] jormungandr_testing_utils::testing::node::RestError),

    #[error(transparent)]
    Startup(#[from] StartupError),
}

pub type Result<T> = ::core::result::Result<T, Error>;

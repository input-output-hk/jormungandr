#![allow(clippy::derive_partial_eq_without_eq)]

mod configuration;
pub mod explorer;
mod fragment_node;
mod legacy;
mod logger;
mod process;
mod remote;
mod rest;
mod starter;
mod verifier;

pub mod grpc;

pub use self::{
    configuration::{
        get_available_port, Block0ConfigurationBuilder, ConfigurationBuilder, JormungandrParams,
        NodeConfigBuilder, SecretModelFactory, TestConfig,
    },
    explorer::{
        compare_schema as compare_explorer_schema, Explorer, ExplorerError, ExplorerProcess,
    },
    fragment_node::{FragmentNode, FragmentNodeError, MemPoolCheck},
    legacy::{
        download_last_n_releases, get_jormungandr_bin, version_0_8_19, BackwardCompatibleRest,
        LegacyConfigConverter, LegacyConfigConverterError, LegacyNodeConfig,
        LegacyNodeConfigConverter, Version,
    },
    logger::{JormungandrLogger, Level as LogLevel},
    process::*,
    remote::{RemoteJormungandr, RemoteJormungandrBuilder},
    rest::{uri_from_socket_addr, JormungandrRest, RawRest, RestError, RestSettings},
    starter::{
        ConfiguredStarter, FaketimeConfig, LeadershipMode, PersistenceMode, Starter, StartupError,
        StartupVerificationMode, TestingDirectory,
    },
    verifier::{assert_accepted_rejected, assert_bad_request, JormungandrStateVerifier},
};
use thiserror::Error;
pub type NodeAlias = String;

#[derive(Error, Debug)]
pub enum JormungandrError {
    #[error("error in logs. Error lines: {error_lines}, full content:{logs}")]
    ErrorInLogs { logs: String, error_lines: String },
    #[error("error in stderr: {stderr}")]
    StdErr { stderr: String },
    #[error("error(s) in log detected: port already in use")]
    PortAlreadyInUse,
}

use std::sync::Arc;

use crate::{
    blockchain::{Blockchain, Tip},
    diagnostic::Diagnostic,
    intercom::{NetworkMsg, TransactionMsg},
    leadership::Logs as LeadershipLogs,
    network::p2p::P2pTopology,
    rest::ServerStopper,
    secure::enclave::Enclave,
    stats_counter::StatsCounter,
    utils::async_msg::MessageBox,
};
use jormungandr_lib::interfaces::NodeState;

use slog::Logger;
use tokio02::sync::RwLock;

#[derive(Clone)]
pub struct Context {
    full: Arc<RwLock<Option<Arc<FullContext>>>>,
    server_stopper: Arc<RwLock<Option<ServerStopper>>>,
    node_state: Arc<RwLock<NodeState>>,
    logger: Arc<RwLock<Option<Logger>>>,
    diagnostic: Arc<RwLock<Option<Diagnostic>>>,
    blockchain: Arc<RwLock<Option<Blockchain>>>,
    blockchain_tip: Arc<RwLock<Option<Tip>>>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Full REST context not available yet")]
    FullContext,
    #[error("Server stopper not set in REST context")]
    ServerStopper,
    #[error("Logger not set in REST context")]
    Logger,
    #[error("Blockchain not set in REST context")]
    Blockchain,
    #[error("Blockchain tip not set in REST context")]
    BlockchainTip,
    #[error("Diagnostic data not set in REST context")]
    Diagnostic,
}

impl Context {
    pub fn new() -> Self {
        Context {
            full: Default::default(),
            server_stopper: Default::default(),
            node_state: Arc::new(RwLock::new(NodeState::StartingRestServer)),
            logger: Default::default(),
            diagnostic: Default::default(),
            blockchain: Default::default(),
            blockchain_tip: Default::default(),
        }
    }

    pub async fn set_full(&self, full_context: FullContext) {
        *self.full.write().await = Some(Arc::new(full_context));
    }

    pub async fn try_full(&self) -> Result<Arc<FullContext>, Error> {
        self.full.read().await.clone().ok_or(Error::FullContext)
    }

    pub async fn set_server_stopper(&self, server_stopper: ServerStopper) {
        *self.server_stopper.write().await = Some(server_stopper);
    }

    pub async fn server_stopper(&self) -> Result<ServerStopper, Error> {
        self.server_stopper
            .read()
            .await
            .clone()
            .ok_or(Error::ServerStopper)
    }

    pub async fn set_node_state(&self, node_state: NodeState) {
        *self.node_state.write().await = node_state;
    }

    pub async fn node_state(&self) -> NodeState {
        self.node_state.read().await.clone()
    }

    pub async fn set_logger(&self, logger: Logger) {
        *self.logger.write().await = Some(logger);
    }

    pub async fn logger(&self) -> Result<Logger, Error> {
        self.logger.read().await.clone().ok_or(Error::Logger)
    }

    pub async fn set_diagnostic_data(&self, diagnostic: Diagnostic) {
        *self.diagnostic.write().await = Some(diagnostic);
    }

    pub async fn get_diagnostic_data(&self) -> Result<Diagnostic, Error> {
        self.diagnostic
            .read()
            .await
            .clone()
            .ok_or(Error::Diagnostic)
    }

    pub async fn set_blockchain(&self, blockchain: Blockchain) {
        *self.blockchain.write().await = Some(blockchain)
    }

    pub async fn blockchain(&self) -> Result<Blockchain, Error> {
        self.blockchain
            .read()
            .await
            .clone()
            .ok_or(Error::Blockchain)
    }

    pub async fn set_blockchain_tip(&self, blockchain_tip: Tip) {
        *self.blockchain_tip.write().await = Some(blockchain_tip)
    }

    pub async fn blockchain_tip(&self) -> Result<Tip, Error> {
        self.blockchain_tip
            .read()
            .await
            .clone()
            .ok_or(Error::BlockchainTip)
    }
}

#[derive(Clone)]
pub struct FullContext {
    pub stats_counter: StatsCounter,
    pub network_task: MessageBox<NetworkMsg>,
    pub transaction_task: MessageBox<TransactionMsg>,
    pub leadership_logs: LeadershipLogs,
    pub enclave: Enclave,
    pub p2p: P2pTopology,
    pub explorer: Option<crate::explorer::Explorer>,
}

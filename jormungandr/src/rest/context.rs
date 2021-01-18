use std::sync::Arc;

use crate::{
    blockchain::{Blockchain, Tip},
    diagnostic::Diagnostic,
    intercom::{NetworkMsg, TransactionMsg},
    leadership::Logs as LeadershipLogs,
    network::GlobalStateR as NetworkStateR,
    rest::ServerStopper,
    secure::enclave::Enclave,
    stats_counter::StatsCounter,
    utils::async_msg::MessageBox,
};
use jormungandr_lib::interfaces::NodeState;

use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::Span;

pub type ContextLock = Arc<RwLock<Context>>;

pub struct Context {
    full: Option<FullContext>,
    server_stopper: Option<ServerStopper>,
    node_state: NodeState,
    span: Option<Span>,
    diagnostic: Option<Diagnostic>,
    blockchain: Option<Blockchain>,
    blockchain_tip: Option<Tip>,
    bootstrap_stopper: Option<CancellationToken>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Full REST context not available yet")]
    FullContext,
    #[error("Server stopper not set in REST context")]
    ServerStopper,
    #[error("Log span not set in REST context")]
    Span,
    #[error("Blockchain not set in REST context")]
    Blockchain,
    #[error("Blockchain tip not set in REST context")]
    BlockchainTip,
    #[error("Diagnostic data not set in REST context")]
    Diagnostic,
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    pub fn new() -> Self {
        Context {
            full: Default::default(),
            server_stopper: Default::default(),
            node_state: NodeState::StartingRestServer,
            span: Default::default(),
            diagnostic: Default::default(),
            blockchain: Default::default(),
            blockchain_tip: Default::default(),
            bootstrap_stopper: Default::default(),
        }
    }

    pub fn set_full(&mut self, full_context: FullContext) {
        self.full = Some(full_context);
    }

    pub fn try_full(&self) -> Result<&FullContext, Error> {
        self.full.as_ref().ok_or(Error::FullContext)
    }

    pub fn set_server_stopper(&mut self, server_stopper: ServerStopper) {
        self.server_stopper = Some(server_stopper);
    }

    pub fn server_stopper(&self) -> Result<&ServerStopper, Error> {
        self.server_stopper.as_ref().ok_or(Error::ServerStopper)
    }

    pub fn set_node_state(&mut self, node_state: NodeState) {
        self.node_state = node_state;
    }

    pub fn node_state(&self) -> &NodeState {
        &self.node_state
    }

    pub fn set_span(&mut self, span: Span) {
        self.span = Some(span);
    }

    pub fn span(&self) -> Result<&Span, Error> {
        self.span.as_ref().ok_or(Error::Span)
    }

    pub fn set_diagnostic_data(&mut self, diagnostic: Diagnostic) {
        self.diagnostic = Some(diagnostic);
    }

    pub fn get_diagnostic_data(&self) -> Result<&Diagnostic, Error> {
        self.diagnostic.as_ref().ok_or(Error::Diagnostic)
    }

    pub fn set_blockchain(&mut self, blockchain: Blockchain) {
        self.blockchain = Some(blockchain)
    }

    pub fn blockchain(&self) -> Result<&Blockchain, Error> {
        self.blockchain.as_ref().ok_or(Error::Blockchain)
    }

    pub fn set_blockchain_tip(&mut self, blockchain_tip: Tip) {
        self.blockchain_tip = Some(blockchain_tip)
    }

    pub fn blockchain_tip(&self) -> Result<&Tip, Error> {
        self.blockchain_tip.as_ref().ok_or(Error::BlockchainTip)
    }

    pub fn set_bootstrap_stopper(&mut self, bootstrap_stopper: CancellationToken) {
        self.bootstrap_stopper = Some(bootstrap_stopper);
    }

    pub fn remove_bootstrap_stopper(&mut self) {
        self.bootstrap_stopper = None;
    }

    pub fn stop_bootstrap(&mut self) {
        if let Some(cancellation_token) = self.bootstrap_stopper.take() {
            cancellation_token.cancel();
        }
    }
}

pub struct FullContext {
    pub stats_counter: StatsCounter,
    pub network_task: MessageBox<NetworkMsg>,
    pub transaction_task: MessageBox<TransactionMsg>,
    pub leadership_logs: LeadershipLogs,
    pub enclave: Enclave,
    pub network_state: NetworkStateR,
    pub explorer: Option<crate::explorer::Explorer>,
}

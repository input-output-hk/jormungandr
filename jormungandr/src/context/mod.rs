use crate::{
    blockchain::{Blockchain, Tip},
    diagnostic::Diagnostic,
    intercom::{NetworkMsg, TopologyMsg, TransactionMsg},
    leadership::Logs as LeadershipLogs,
    metrics::backends::SimpleCounter,
    network::GlobalStateR as NetworkStateR,
    secure::enclave::Enclave,
    utils::async_msg::MessageBox,
};
use futures::channel::mpsc;
use jormungandr_lib::interfaces::NodeState;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::Span;

pub type ContextLock = Arc<RwLock<Context>>;

#[derive(Clone)]
pub struct ServerStopper(mpsc::Sender<()>);

impl ServerStopper {
    pub fn new(sender: mpsc::Sender<()>) -> Self {
        Self(sender)
    }

    pub fn stop(&self) {
        self.0.clone().try_send(()).unwrap();
    }
}

pub struct Context {
    full: Option<FullContext>,
    rest_server_stopper: Option<ServerStopper>,
    node_state: NodeState,
    span: Option<Span>,
    diagnostic: Option<Diagnostic>,
    blockchain: Option<Blockchain>,
    blockchain_tip: Option<Tip>,
    bootstrap_stopper: Option<CancellationToken>,
    #[cfg(feature = "evm")]
    evm_filters: crate::jrpc::EvmFilters,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Full REST/RPC context not available yet")]
    FullContext,
    #[error("Server stopper not set in REST/RPC context")]
    ServerStopper,
    #[error("Log span not set in REST/RPC context")]
    Span,
    #[error("Blockchain not set in REST/RPC context")]
    Blockchain,
    #[error("Blockchain tip not set in REST/RPC context")]
    BlockchainTip,
    #[error("Diagnostic data not set in REST/RPC context")]
    Diagnostic,
}

impl warp::reject::Reject for Error {}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    pub fn new() -> Self {
        Context {
            full: Default::default(),
            rest_server_stopper: Default::default(),
            node_state: NodeState::StartingRestServer,
            span: Default::default(),
            diagnostic: Default::default(),
            blockchain: Default::default(),
            blockchain_tip: Default::default(),
            bootstrap_stopper: Default::default(),
            #[cfg(feature = "evm")]
            evm_filters: Default::default(),
        }
    }

    pub fn set_full(&mut self, full_context: FullContext) {
        self.full = Some(full_context);
    }

    pub fn try_full(&self) -> Result<&FullContext, Error> {
        self.full.as_ref().ok_or(Error::FullContext)
    }

    pub fn set_rest_server_stopper(&mut self, server_stopper: ServerStopper) {
        self.rest_server_stopper = Some(server_stopper);
    }

    pub fn rest_server_stopper(&self) -> Result<&ServerStopper, Error> {
        self.rest_server_stopper
            .as_ref()
            .ok_or(Error::ServerStopper)
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

    #[cfg(feature = "evm")]
    pub fn evm_filters(&mut self) -> &mut crate::jrpc::EvmFilters {
        &mut self.evm_filters
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
    pub stats_counter: Arc<SimpleCounter>,
    pub network_task: MessageBox<NetworkMsg>,
    pub topology_task: MessageBox<TopologyMsg>,
    pub transaction_task: MessageBox<TransactionMsg>,
    pub leadership_logs: LeadershipLogs,
    pub enclave: Enclave,
    #[cfg(feature = "evm")]
    pub evm_keys: Arc<Vec<chain_evm::util::Secret>>,
    pub network_state: NetworkStateR,
    #[cfg(feature = "prometheus-metrics")]
    pub prometheus: Option<Arc<crate::metrics::backends::Prometheus>>,
}

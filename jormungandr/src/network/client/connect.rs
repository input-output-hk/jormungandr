use super::{Client, ClientBuilder, InboundSubscriptions};
use crate::blockcfg::HeaderHash;
use crate::network::{
    grpc,
    p2p::{comm::PeerComms, Address},
    security_params::NONCE_LEN,
    Channels, ConnectionState,
};
use chain_core::mempack::{self, ReadBuf, Readable};
use chain_network::data::{AuthenticatedNodeId, NodeId};
use chain_network::error::{self as net_error, HandshakeError};
use chain_network::grpc::legacy;

use futures::channel::oneshot;
use futures::future::BoxFuture;
use futures::prelude::*;
use futures::ready;
use rand::Rng;

use std::convert::TryInto;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Initiates a client connection, returning a connection handle and
/// the connection future that must be polled to complete the connection.
///
/// Note that this is the only function in this module that is tied to the
/// gRPC protocol, all other code is generic in terms of network-core traits.
/// This is intentional, to facilitate extension to different protocols
/// in the future.
pub fn connect(state: ConnectionState, channels: Channels) -> (ConnectHandle, ConnectFuture) {
    let (sender, receiver) = oneshot::channel();
    let peer = state.peer();
    let keypair = state.global.keypair.clone();
    let legacy_node_id = state.global.config.legacy_node_id;
    let _enter = state.span().enter();
    let cf = async move {
        let mut grpc_client = if let Some(node_id) = legacy_node_id {
            let node_id: legacy::NodeId = node_id.as_ref().try_into().unwrap();
            tracing::debug!(
                "connecting with legacy node id {}",
                hex::encode(node_id.as_bytes())
            );
            grpc::connect_legacy(&peer, node_id).await
        } else {
            tracing::debug!("connecting");
            grpc::connect(&peer).await
        }
        .map_err(ConnectError::Transport)?;

        let mut nonce = [0u8; NONCE_LEN];
        rand::thread_rng().fill(&mut nonce);

        let hr = grpc_client
            .handshake(&nonce[..])
            .await
            .map_err(ConnectError::Handshake)?;
        let mut buf = ReadBuf::from(hr.block0_id.as_bytes());
        let block0_hash = HeaderHash::read(&mut buf).map_err(ConnectError::DecodeBlock0)?;
        let expected = state.global.block0_hash;
        match_block0(expected, block0_hash)?;

        // Validate the server's node ID
        let peer_id = validate_peer_auth(hr.auth, &nonce)?;

        tracing::debug!(node_id = ?peer_id, "authenticated server peer node");

        // Send client authentication
        let auth = keypair.sign(&hr.nonce);
        grpc_client
            .client_auth(auth)
            .await
            .map_err(ConnectError::ClientAuth)?;

        let mut comms = PeerComms::new();
        comms.set_node_id(peer_id);
        let (block_sub, fragment_sub, gossip_sub) = future::try_join3(
            grpc_client
                .clone()
                .block_subscription(comms.subscribe_to_block_announcements()),
            grpc_client
                .clone()
                .fragment_subscription(comms.subscribe_to_fragments()),
            grpc_client
                .clone()
                .gossip_subscription(comms.subscribe_to_gossip()),
        )
        .await
        .map_err(ConnectError::Subscription)?;
        let inbound = InboundSubscriptions {
            peer_address: Address::tcp(peer.connection),
            block_events: block_sub,
            fragments: fragment_sub,
            gossip: gossip_sub,
        };
        let builder = ClientBuilder { channels, logger };
        let client = Client::new(
            grpc_client,
            builder,
            state.global.clone(),
            inbound,
            &mut comms,
        );
        Ok((client, comms))
    };
    let handle = ConnectHandle { receiver };
    let future = ConnectFuture {
        sender: Some(sender),
        task: cf.boxed(),
    };
    (handle, future)
}

// Validate the server peer's node ID
fn validate_peer_auth(auth: AuthenticatedNodeId, nonce: &[u8]) -> Result<NodeId, ConnectError> {
    auth.verify(&nonce)
        .map_err(ConnectError::PeerSignatureVerificationFailed)?;
    Ok(auth.into())
}

/// Handle used to monitor the P2P client in process of
/// establishing a connection and subscription streams.
///
/// If the handle is dropped before the connection is established,
/// the client connection is canceled.
pub struct ConnectHandle {
    receiver: oneshot::Receiver<PeerComms>,
}

/// An error type to signal that the connection was not established.
/// The reason should be logged already, so this error type should not be
/// used for reporting.
pub type ConnectCanceled = oneshot::Canceled;

impl ConnectHandle {
    /// Checks if the client has connected and established subscriptions,
    /// and if so, returns the communication handles.
    ///
    /// This method does not use a task context and does not schedule a wakeup.
    pub fn try_complete(&mut self) -> Result<Option<PeerComms>, ConnectCanceled> {
        self.receiver.try_recv()
    }
}

/// The future that drives P2P client to establish a connection.
#[must_use = "futures do nothing unless polled"]
pub struct ConnectFuture {
    sender: Option<oneshot::Sender<PeerComms>>,
    task: BoxFuture<'static, Result<(Client, PeerComms), ConnectError>>,
}

#[allow(dead_code)]
#[derive(thiserror::Error, Debug)]
pub enum ConnectError {
    #[error("connection has been canceled")]
    Canceled,
    #[error(transparent)]
    Transport(tonic::transport::Error),
    #[error("protocol handshake failed: {0}")]
    Handshake(#[source] HandshakeError),
    #[error("failed to decode genesis block in response")]
    DecodeBlock0(#[source] mempack::ReadError),
    #[error(
        "genesis block hash {peer_responded} reported by the peer is not the expected {expected}"
    )]
    Block0Mismatch {
        expected: HeaderHash,
        peer_responded: HeaderHash,
    },
    #[error("invalid node ID in server Handshake response")]
    InvalidNodeId(#[source] chain_crypto::PublicKeyError),
    #[error("invalid signature data in server Handshake response")]
    InvalidNodeSignature(#[source] chain_crypto::SignatureError),
    #[error("signature verification failed for peer node ID")]
    PeerSignatureVerificationFailed(#[source] net_error::Error),
    #[error("client authentication failed")]
    ClientAuth(#[source] net_error::Error),
    #[error("subscription request failed")]
    Subscription(#[source] net_error::Error),
}

impl Future for ConnectFuture {
    type Output = Result<Client, ConnectError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // First, check if the connection is cancelled
        if let Poll::Ready(()) = self
            .sender
            .as_mut()
            .expect("polled a future after it has been resolved")
            .poll_canceled(cx)
        {
            return Err(ConnectError::Canceled).into();
        }

        let (client, comms) = ready!(Pin::new(&mut self.task).poll(cx))?;

        match self.sender.take().unwrap().send(comms) {
            Ok(()) => Ok(client).into(),
            Err(_) => Err(ConnectError::Canceled).into(),
        }
    }
}

fn match_block0(expected: HeaderHash, peer_responded: HeaderHash) -> Result<(), ConnectError> {
    if expected == peer_responded {
        Ok(())
    } else {
        Err(ConnectError::Block0Mismatch {
            expected,
            peer_responded,
        })
    }
}

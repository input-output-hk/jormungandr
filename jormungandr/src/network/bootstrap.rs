use super::grpc;
use crate::{
    blockchain::{self, Blockchain, BootstrapError, Error as BlockchainError, Tip},
    network::convert::Decode,
    settings::start::network::Peer,
    topology,
};
use chain_core::property::ReadError;
use chain_network::{data as net_data, error::Error as NetworkError};
use futures::prelude::*;
use std::fmt::Debug;
use tokio_util::sync::CancellationToken;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed to connect to bootstrap peer")]
    Connect(#[source] grpc::ConnectError),
    #[error("connection broken")]
    ClientNotReady(#[source] NetworkError),
    #[error("peers not available")]
    PeersNotAvailable(#[source] NetworkError),
    #[error("bootstrap pull request failed")]
    PullRequestFailed(#[source] NetworkError),
    #[error("could not get the blockchain tip from a peer")]
    TipFailed(#[source] NetworkError),
    #[error(transparent)]
    PeerDecodingFailed(NetworkError),
    #[error("decoding of a block failed")]
    BlockDecodingFailed(#[source] ReadError),
    #[error(transparent)]
    Blockchain(#[from] Box<BootstrapError>),
    #[error("failed to collect garbage and flush blocks to the permanent storage")]
    GcFailed(#[source] Box<BlockchainError>),
    #[error("bootstrap pull stream failed")]
    PullStreamFailed(#[source] NetworkError),
    #[error("Trusted peers cannot be empty. To avoid bootstrap use `skip_bootstrap: true`")]
    EmptyTrustedPeers,
    #[error("the bootstrap process was interrupted")]
    Interrupted,
}

const MAX_BOOTSTRAP_PEERS: u32 = 32;

pub async fn peers_from_trusted_peer(peer: &Peer) -> Result<Vec<topology::Peer>, Error> {
    tracing::info!("getting peers from bootstrap peer {}", peer.connection);

    let mut client = grpc::connect(peer).await.map_err(Error::Connect)?;
    let gossip = client
        .peers(MAX_BOOTSTRAP_PEERS)
        .await
        .map_err(Error::PeersNotAvailable)?;
    let peers = gossip
        .nodes
        .decode()
        .map_err(Error::PeerDecodingFailed)?
        .into_iter()
        .map(topology::Peer::from)
        .collect::<Vec<_>>();

    tracing::info!("peer {} : peers known : {}", peer.connection, peers.len());
    Ok(peers)
}

pub async fn bootstrap_from_peer(
    peer: &Peer,
    blockchain: Blockchain,
    tip: Tip,
    cancellation_token: CancellationToken,
) -> Result<(), Error> {
    use chain_network::data::BlockId;

    async fn with_cancellation_token<T>(
        future: impl Future<Output = T> + Unpin,
        token: &CancellationToken,
    ) -> Result<T, Error> {
        use futures::future::{select, Either};

        match select(future, token.cancelled().boxed()).await {
            Either::Left((result, _)) => Ok(result),
            Either::Right(((), _)) => Err(Error::Interrupted),
        }
    }

    tracing::debug!("connecting to bootstrap peer {}", peer.connection);

    let mut client = with_cancellation_token(grpc::connect(peer).boxed(), &cancellation_token)
        .await?
        .map_err(Error::Connect)?;

    loop {
        let remote_tip = with_cancellation_token(client.tip().boxed(), &cancellation_token)
            .await?
            .and_then(|header| header.decode())
            .map_err(Error::TipFailed)?
            .id();

        if remote_tip == tip.get_ref().await.hash() {
            break Ok(());
        }

        let checkpoints = blockchain.get_checkpoints(&tip.branch().await);
        let checkpoints = net_data::block::try_ids_from_iter(checkpoints).unwrap();

        let remote_tip = BlockId::try_from(remote_tip.as_ref()).unwrap();

        tracing::info!(
            "pulling blocks starting from checkpoints: {:?}; to tip {:?}",
            checkpoints,
            remote_tip,
        );

        let stream = with_cancellation_token(
            client.pull_blocks(checkpoints, remote_tip).boxed(),
            &cancellation_token,
        )
        .await?
        .map_err(Error::PullRequestFailed)?;

        blockchain::bootstrap_from_stream(
            blockchain.clone(),
            tip.clone(),
            stream,
            cancellation_token.clone(),
        )
        .await
        .map_err(Box::new)?;
    }
}

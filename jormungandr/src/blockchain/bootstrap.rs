use super::tip::TipUpdater;
use crate::blockcfg::{Block, HeaderDesc, HeaderHash};
use crate::blockchain::{self, Blockchain, PreCheckedHeader, Ref, Tip};
use crate::metrics::Metrics;
use chain_core::property::{Deserialize, HasHeader};
use chain_network::data as net_data;
use chain_network::error::Error as NetworkError;
use futures::{prelude::*, stream, task::Poll};
use tokio_util::sync::CancellationToken;

use std::pin::Pin;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    BlockchainError(#[from] super::Error),
    #[error(
        "received block {0} is already present, but does not descend from any of the checkpoints"
    )]
    BlockNotOnBranch(HeaderHash),
    #[error("received block {0} is not connected to the block chain")]
    BlockMissingParent(HeaderHash),
    #[error("bootstrap pull stream failed")]
    PullStreamFailed(#[source] NetworkError),
    #[error("failures while deserializing block from stream")]
    BlockDeserialize(#[from] std::io::Error),
    #[error("the bootstrap process was interrupted")]
    Interrupted,
}

pub async fn bootstrap_from_stream<S>(
    blockchain: Blockchain,
    branch: Tip,
    stream: S,
    cancellation_token: CancellationToken,
) -> Result<Option<Arc<Ref>>, Error>
where
    S: Stream<Item = Result<net_data::Block, NetworkError>> + Unpin,
{
    const PROCESS_LOGGING_DISTANCE: u64 = 2500;
    let block0 = *blockchain.block0();
    let mut tip_updater = TipUpdater::new(
        branch,
        blockchain.clone(),
        None,
        None,
        Metrics::builder().build(),
    );

    let mut bootstrap_info = BootstrapInfo::new();
    let mut maybe_parent_tip = None;

    // This stream will either end when the block stream is exhausted or when
    // the cancellation signal arrives. Building such stream allows us to
    // correctly write all blocks and update the block tip upon the arrival of
    // the cancellation signal.
    let mut cancel = cancellation_token.cancelled().boxed();
    let mut stream = stream.map_err(Error::PullStreamFailed);

    let mut stream = stream::poll_fn(move |cx| {
        let cancel = Pin::new(&mut cancel);
        match cancel.poll(cx) {
            Poll::Pending => {
                let stream = Pin::new(&mut stream);
                stream.poll_next(cx)
            }
            Poll::Ready(()) => Poll::Ready(Some(Err(Error::Interrupted))),
        }
    });

    while let Some(block_result) = stream.next().await {
        let result = match block_result {
            Ok(block) => {
                let block = Block::deserialize(block.as_bytes())?;

                if block.header.hash() == block0 {
                    continue;
                }

                bootstrap_info.append_block(&block);

                if bootstrap_info.block_received % PROCESS_LOGGING_DISTANCE == 0 {
                    bootstrap_info.report();
                }

                handle_block(&blockchain, block).await
            }
            Err(err) => Err(err),
        };

        match result {
            Ok(parent_tip) => {
                maybe_parent_tip = Some(parent_tip);
            }
            Err(err) => {
                if let Some(bootstrap_tip) = maybe_parent_tip {
                    tip_updater.process_new_ref(bootstrap_tip).await?;
                }
                return Err(err);
            }
        }
    }

    if let Some(ref bootstrap_tip) = maybe_parent_tip {
        tip_updater.process_new_ref(bootstrap_tip.clone()).await?;
    } else {
        tracing::info!("no new blocks in bootstrap stream");
    }

    Ok(maybe_parent_tip)
}

async fn handle_block(blockchain: &Blockchain, block: Block) -> Result<Arc<Ref>, Error> {
    let header = block.header();
    let pre_checked = blockchain.pre_check_header(header, true).await?;
    match pre_checked {
        PreCheckedHeader::AlreadyPresent {
            cached_reference: Some(block_ref),
            ..
        } => Ok(block_ref),
        PreCheckedHeader::AlreadyPresent {
            cached_reference: None,
            header,
        } => Err(Error::BlockNotOnBranch(header.hash())),
        PreCheckedHeader::MissingParent { header, .. } => {
            Err(Error::BlockMissingParent(header.hash()))
        }
        PreCheckedHeader::HeaderWithCache { header, parent_ref } => {
            let post_checked = blockchain
                .post_check_header(header, parent_ref, blockchain::CheckHeaderProof::Enabled)
                .await?;

            tracing::debug!(
                hash = %post_checked.header().hash(),
                block_date = %post_checked.header().block_date(),
                "validated block"
            );
            let applied = blockchain
                .apply_and_store_block(post_checked, block)
                .await?;
            Ok(applied.cached_ref())
        }
    }
}

struct BootstrapInfo {
    last_reported: std::time::SystemTime,
    last_bytes_received: u64,
    bytes_received: u64,
    block_received: u64,
    last_block_description: Option<HeaderDesc>,
}

impl BootstrapInfo {
    pub fn new() -> Self {
        let now = std::time::SystemTime::now();
        let lbd: Option<HeaderDesc> = None;
        BootstrapInfo {
            last_reported: now,
            last_bytes_received: 0,
            bytes_received: 0,
            block_received: 0,
            last_block_description: lbd,
        }
    }

    pub fn append_block(&mut self, b: &Block) {
        use chain_core::property::Serialize;
        self.bytes_received += b.serialize_as_vec().unwrap().len() as u64; // TODO sad serialization back
        self.block_received += 1;
        self.last_block_description = Some(b.header.description());
    }

    pub fn report(&mut self) {
        fn print_sz(n: f64) -> String {
            if n > 1_000_000.0 {
                format!("{:.2}mb", n / (1024 * 1024) as f64)
            } else if n > 1_000.0 {
                format!("{:.2}kb", n / 1024_f64)
            } else {
                format!("{:.2}b", n)
            }
        }
        let current = std::time::SystemTime::now();
        let time_diff = current.duration_since(self.last_reported);
        let bytes_diff = self.bytes_received - self.last_bytes_received;

        let bytes = print_sz(bytes_diff as f64);
        let kbs = time_diff
            .map(|td| {
                let v = (bytes_diff as f64) / td.as_secs_f64();
                print_sz(v)
            })
            .unwrap_or_else(|_| "N/A".to_string());

        self.last_reported = current;
        self.last_bytes_received = self.bytes_received;
        tracing::info!(
            "receiving from network bytes={} {}/s, blockchain {}",
            bytes,
            kbs,
            self.last_block_description
                .as_ref()
                .map(|lbd| lbd.to_string())
                .expect("append_block should always be called before report")
        )
    }
}

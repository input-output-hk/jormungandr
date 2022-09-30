use super::tip::TipUpdater;
use crate::{
    blockcfg::{Block, HeaderHash},
    blockchain::{
        chain::{CheckHeaderProof, StreamInfo, StreamReporter},
        Blockchain, Ref, Tip,
    },
    metrics::Metrics,
};
use chain_core::{
    packer::Codec,
    property::{Deserialize, ReadError},
};
use chain_network::{data as net_data, error::Error as NetworkError};
use futures::prelude::*;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    BlockchainError(Box<super::Error>),
    #[error("received block {0} is not connected to the block chain")]
    BlockMissingParent(HeaderHash),
    #[error("bootstrap pull stream failed")]
    PullStreamFailed(#[source] NetworkError),
    #[error("failures while deserializing block from stream")]
    BlockDeserialize(#[from] ReadError),
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
    let block0 = *blockchain.block0();
    let mut tip_updater = TipUpdater::new(
        branch,
        blockchain.clone(),
        None,
        None,
        Metrics::builder().build(),
    );

    let mut bootstrap_info = StreamReporter::new(report);
    let mut maybe_parent_tip = None;

    // This stream will either end when the block stream is exhausted or when
    // the cancellation signal arrives. Building such stream allows us to
    // correctly write all blocks and update the block tip upon the arrival of
    // the cancellation signal.
    let cancel = cancellation_token.cancelled();
    tokio::pin!(cancel);
    let mut stream = stream
        .map_err(Error::PullStreamFailed)
        .map(|maybe_block| {
            maybe_block.and_then(|b| Ok(Block::deserialize(&mut Codec::new(b.as_bytes()))?))
        })
        .take_until(cancel);

    while let Some(block_result) = stream.next().await {
        let maybe_tip = match block_result {
            Ok(block) => {
                if block.header().hash() == block0 {
                    continue;
                }

                bootstrap_info.append_block(&block);
                blockchain
                    .handle_bootstrap_block(block, CheckHeaderProof::Enabled)
                    .await
                    .map_err(|e| Error::BlockchainError(Box::new(e)))
            }
            Err(err) => Err(err),
        };
        match maybe_tip {
            Ok(parent_tip) => {
                maybe_parent_tip = Some(parent_tip);
            }
            Err(err) => {
                if let Some(bootstrap_tip) = maybe_parent_tip {
                    tip_updater
                        .process_new_ref(bootstrap_tip)
                        .await
                        .map_err(|e| Error::BlockchainError(Box::new(e)))?;
                }
                return Err(err);
            }
        }
    }

    if let Some(ref bootstrap_tip) = maybe_parent_tip {
        tip_updater
            .process_new_ref(bootstrap_tip.clone())
            .await
            .map_err(|e| Error::BlockchainError(Box::new(e)))?;
    } else {
        tracing::info!("no new blocks received from the network");
    }

    if stream.take_result().is_some() {
        return Err(Error::Interrupted);
    }

    Ok(maybe_parent_tip)
}

fn report(stream_info: &StreamInfo) {
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
    let time_diff = current.duration_since(stream_info.last_reported);
    let bytes_diff = stream_info.bytes_received - stream_info.last_bytes_received;

    let bytes = print_sz(bytes_diff as f64);
    let kbs = time_diff
        .map(|td| {
            let v = (bytes_diff as f64) / td.as_secs_f64();
            print_sz(v)
        })
        .unwrap_or_else(|_| "N/A".to_string());

    tracing::info!(
        "receiving from network bytes={} {}/s, blockchain {}",
        bytes,
        kbs,
        stream_info
            .last_block_description
            .as_ref()
            .map(|lbd| lbd.to_string())
            .expect("append_block should always be called before report")
    )
}

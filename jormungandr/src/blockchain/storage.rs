use crate::{
    blockcfg::{Block, HeaderHash},
    intercom::{self, ReplySendError, ReplyStreamHandle},
};
use chain_core::{
    packer::Codec,
    property::{Deserialize, ReadError, Serialize, WriteError},
};
use chain_storage::{BlockInfo, BlockStore, Error as StorageError};
use futures::prelude::*;
use std::{convert::identity, path::Path};
use thiserror::Error;
use tracing::Span;

const MINIMUM_BLOCKS_TO_FLUSH: usize = 256;

#[derive(Debug, Error)]
pub enum Error {
    #[error("block not found")]
    BlockNotFound,
    #[error("database backend error")]
    BackendError(#[source] StorageError),
    #[error("deserialization error")]
    Deserialize(#[source] ReadError),
    #[error("serialization error")]
    Serialize(#[source] WriteError),
    #[error("Block already present in DB")]
    BlockAlreadyPresent,
    #[error("the parent block is missing for the required write")]
    MissingParent,
    #[error("cannot iterate between the 2 given blocks")]
    CannotIterate,
}

impl From<StorageError> for Error {
    fn from(source: StorageError) -> Self {
        match source {
            StorageError::BlockNotFound => Error::BlockNotFound,
            StorageError::BlockAlreadyPresent => Error::BlockAlreadyPresent,
            StorageError::MissingParent => Error::MissingParent,
            e => Error::BackendError(e),
        }
    }
}

#[derive(Clone)]
pub struct Storage {
    storage: BlockStore,
    span: Span,
}

pub struct Ancestor {
    pub header_hash: HeaderHash,
    pub distance: u32,
}

#[derive(Debug, thiserror::Error)]
enum StreamingError {
    #[error("error accessing storage")]
    Storage(#[from] Error),
    #[error("failed to send block")]
    Sending(#[from] ReplySendError),
}

impl Storage {
    pub fn file<P: AsRef<Path>>(path: P, span: Span) -> Result<Self, Error> {
        let storage = BlockStore::file(path, HeaderHash::zero_hash().as_bytes().to_vec())?;
        Ok(Storage { storage, span })
    }

    pub fn memory(span: Span) -> Result<Self, Error> {
        let storage = BlockStore::memory(HeaderHash::zero_hash().as_bytes().to_vec())?;
        Ok(Storage { storage, span })
    }

    pub fn get_tag(&self, tag: &str) -> Result<Option<HeaderHash>, Error> {
        self.storage
            .get_tag(tag)
            .map_err(Into::into)
            .and_then(|maybe_block_id| {
                maybe_block_id
                    .map(|block_id| {
                        HeaderHash::deserialize(&mut Codec::new(block_id.as_ref()))
                            .map_err(Error::Deserialize)
                    })
                    .transpose()
            })
    }

    pub fn put_tag(&self, tag: &str, header_hash: HeaderHash) -> Result<(), Error> {
        self.storage
            .put_tag(tag, header_hash.as_bytes())
            .map_err(Into::into)
    }

    pub fn get(&self, header_hash: HeaderHash) -> Result<Option<Block>, Error> {
        match self.storage.get_block(header_hash.as_bytes()) {
            Ok(block) => Block::deserialize(&mut Codec::new(block.as_ref()))
                .map(Some)
                .map_err(Error::Deserialize),
            Err(StorageError::BlockNotFound) => Ok(None),
            Err(e) => Err(Error::BackendError(e)),
        }
    }

    pub fn block_exists(&self, header_hash: HeaderHash) -> Result<bool, Error> {
        self.storage
            .block_exists(header_hash.as_ref())
            .map_err(Into::into)
    }

    pub fn get_branches(&self) -> Result<Vec<HeaderHash>, Error> {
        self.storage
            .get_tips_ids()?
            .into_iter()
            .map(|branch| {
                HeaderHash::deserialize(&mut Codec::new(branch.as_ref()))
                    .map_err(Error::Deserialize)
            })
            .collect::<Result<Vec<_>, Error>>()
    }

    pub fn get_blocks_by_chain_length(&self, chain_length: u32) -> Result<Vec<Block>, Error> {
        self.storage
            .get_blocks_by_chain_length(chain_length)
            .map_err(Into::into)
            .and_then(|blocks| {
                blocks
                    .into_iter()
                    .map(|block| Block::deserialize(&mut Codec::new(block.as_ref())))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(Error::Deserialize)
            })
    }

    pub fn get_nth_ancestor(
        &self,
        header_hash: HeaderHash,
        distance: u32,
    ) -> Result<Option<Block>, Error> {
        match self
            .storage
            .get_nth_ancestor(header_hash.as_bytes(), distance)
        {
            Ok(block) => {
                let block = self
                    .storage
                    .get_block(block.id().as_ref())
                    .expect("already found this block, it must exists inside the storage");
                Block::deserialize(&mut Codec::new(block.as_ref()))
                    .map(Some)
                    .map_err(Error::Deserialize)
            }
            Err(StorageError::BlockNotFound) => Ok(None),
            Err(e) => Err(Error::BackendError(e)),
        }
    }

    pub fn put_block(&self, block: &Block) -> Result<(), Error> {
        let id = block
            .header()
            .hash()
            .serialize_as_vec()
            .map_err(Error::Serialize)?;
        let parent_id = block
            .header()
            .block_parent_hash()
            .serialize_as_vec()
            .map_err(Error::Serialize)?;
        let chain_length = block.header().chain_length().into();
        let block_info = BlockInfo::new(id, parent_id, chain_length);
        self.storage
            .put_block(
                &block.serialize_as_vec().map_err(Error::Serialize)?[..],
                block_info,
            )
            .map_err(Into::into)
    }

    pub fn get_parent(&self, header_hash: HeaderHash) -> Result<Option<HeaderHash>, Error> {
        let block_info = match self.storage.get_block_info(header_hash.as_ref()) {
            Ok(block_info) => block_info,
            Err(_) => return Ok(None),
        };

        HeaderHash::deserialize(&mut Codec::new(block_info.parent_id().as_ref()))
            .map_err(Error::Deserialize)
            .map(Some)
    }

    pub fn is_ancestor(&self, a: HeaderHash, b: HeaderHash) -> bool {
        self.storage
            .is_ancestor(a.as_ref(), b.as_ref())
            .map(|x| x.is_some())
            .unwrap_or(false)
    }

    pub fn get_chain_length(&self, block_id: HeaderHash) -> Option<u32> {
        let block_info = match self.storage.get_block_info(block_id.as_ref()) {
            Ok(block_info) => block_info,
            Err(_) => return None,
        };

        Some(block_info.chain_length())
    }

    /// Return values:
    /// - `Ok(stream)` - `from` is ancestor of `to`, returns blocks between them
    /// - `Err(CannotIterate)` - `from` is not ancestor of `to`
    /// - `Err(BlockNotFound)` - `from` or `to` was not found
    /// - `Err(_)` - some other storage error
    pub fn stream_from_to(
        &self,
        from: HeaderHash,
        to: HeaderHash,
    ) -> Result<impl Stream<Item = Result<Block, intercom::Error>>, Error> {
        let distance = self
            .storage
            .is_ancestor(from.as_bytes(), to.as_bytes())?
            .ok_or(Error::CannotIterate)?;

        let stream = futures::stream::iter(self.storage.iter(to.as_bytes(), distance)?)
            .map_err(Into::into)
            .and_then(|raw_block| async move {
                Block::deserialize(&mut Codec::new(raw_block.as_ref())).map_err(Error::Deserialize)
            })
            .map_err(Into::into);

        Ok(stream)
    }

    /// Stream a branch ending at `to` and starting from the ancestor
    /// at `depth` or at the first ancestor since genesis block
    /// if `depth` is given as `None`.
    ///
    /// This function uses buffering in the in-memory channel to reduce
    /// synchronization overhead.
    pub async fn send_branch(
        &self,
        to: HeaderHash,
        depth: Option<u32>,
        handle: ReplyStreamHandle<Block>,
    ) -> Result<(), ReplySendError> {
        self.send_branch_with(to, depth, handle, identity).await
    }

    /// Like `send_branch`, but with a transformation function applied
    /// to the block content before sending to the in-memory channel.
    pub async fn send_branch_with<T, F>(
        &self,
        to: HeaderHash,
        depth: Option<u32>,
        handle: ReplyStreamHandle<T>,
        transform: F,
    ) -> Result<(), ReplySendError>
    where
        F: FnMut(Block) -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        let iter_result = self.storage.iter(to.as_bytes(), depth.unwrap_or(1));

        let iter = match iter_result {
            Ok(iter) => iter,
            Err(err) => {
                let err: Error = err.into();
                handle.reply_error(err.into());
                return Ok(());
            }
        };

        let mut stream = futures::stream::iter(iter)
            .map(|raw_block_result| {
                raw_block_result.map_err(Into::into).and_then(|raw_block| {
                    Block::deserialize(&mut Codec::new(raw_block.as_ref()))
                        .map_err(Error::Deserialize)
                })
            })
            .map_ok(transform)
            .map_err(Into::into)
            .map(Ok);

        handle.start_sending().send_all(&mut stream).await
    }

    pub fn find_closest_ancestor(
        &self,
        checkpoints: Vec<HeaderHash>,
        descendant: HeaderHash,
    ) -> Result<Option<Ancestor>, Error> {
        let mut ancestor = None;
        let mut closest_found = std::u32::MAX;

        for checkpoint in checkpoints {
            // Checkpoints sent by a peer may not
            // be present locally, so we need to ignore certain errors
            match self
                .storage
                .is_ancestor(checkpoint.as_bytes(), descendant.as_bytes())
            {
                Ok(None) => {}
                Ok(Some(distance)) => {
                    if closest_found > distance {
                        ancestor = Some(checkpoint);
                        closest_found = distance;
                    }
                }
                Err(e) => {
                    // Checkpoints sent by a peer may not
                    // be present locally, so we need to ignore certain errors
                    match e {
                        StorageError::BlockNotFound => {
                            // FIXME: add block hash into the error so we
                            // can see which of the two it is.
                            // For now, just ignore either.
                        }
                        e => return Err(e.into()),
                    }
                }
            }
        }

        Ok(ancestor.map(|header_hash| Ancestor {
            header_hash,
            distance: closest_found,
        }))
    }

    pub fn find_common_ancestor(
        &self,
        tip_1: HeaderHash,
        tip_2: HeaderHash,
    ) -> Result<HeaderHash, Error> {
        HeaderHash::deserialize(&mut Codec::new(
            self.storage
                .find_lowest_common_ancestor(tip_1.as_ref(), tip_2.as_ref())?
                // No common ancestor means that we accepted blocks originating from two different block0
                .unwrap()
                .id()
                .as_ref(),
        ))
        .map_err(Error::Deserialize)
    }

    pub fn gc(&self, threshold_depth: u32, main_branch_tip: &[u8]) -> Result<(), Error> {
        let _enter = self.span.enter();
        let main_info = self.storage.get_block_info(main_branch_tip)?;
        let threshold_length = match main_info.chain_length().checked_sub(threshold_depth) {
            Some(result) => result,
            None => return Ok(()),
        };

        tracing::debug!(
            "pruning all branches below stability depth {} (chain length: {})",
            threshold_depth,
            threshold_length
        );

        let tips_ids = self.storage.get_tips_ids()?;

        for id in tips_ids {
            let info = self.storage.get_block_info(id.as_ref())?;

            if info.chain_length() > threshold_length {
                continue;
            }

            self.storage.prune_branch(id.as_ref())?;

            tracing::debug!(
                "removed branch with head {}",
                HeaderHash::hash_bytes(id.as_ref())
            );
        }

        let to_block_info = self
            .storage
            .get_nth_ancestor(main_branch_tip, threshold_depth)?;
        let blocks_flushed = self
            .storage
            .flush_to_permanent_store(to_block_info.id().as_ref(), MINIMUM_BLOCKS_TO_FLUSH)?;

        tracing::debug!(
            "flushed all blocks ({}) up to {} to the permanent store",
            blocks_flushed,
            HeaderHash::hash_bytes(to_block_info.id().as_ref())
        );

        Ok(())
    }
}

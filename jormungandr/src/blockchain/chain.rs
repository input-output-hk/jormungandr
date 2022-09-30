/*

```text
          +------------+                     +------------+                    +------------+
          | Leadership |                     | Leadership |                    | Leadership |
          +-----+------+                     +-----+------+                    +-------+----+
                ^                                  ^                                   ^
                |                                  |                                   |
      +---------v-----^--------------+             +<------------+                +--->+--------+
      |               |              |             |             |                |             |
      |               |              |             |             |                |             |
   +--+--+         +--+--+        +--+--+       +--+--+       +--+--+          +--+--+       +--+--+
   | Ref +<--------+ Ref +<-------+ Ref +<--+---+ Ref +<------+ Ref +<---------+ Ref +<------+ Ref |
   +--+--+         +--+--+        +--+--+   ^   +--+--+       +--+--+          +---+-+       +---+-+
      |               |              |      |      |             |                 |             |
      v               v              v      |      v             v                 v             v
+-----+--+      +-----+--+       +---+----+ |   +--+-----+   +---+----+      +-----+--+       +--+-----+
| Ledger |      | Ledger |       | Ledger | |   | Ledger |   | Ledger |      | Ledger |       | Ledger |
+--------+      +--------+       +--------+ |   +--------+   +--------+      +--------+       +--------+
                                            |
                                            |
                                            |parent
                                            |hash
                                            |
                                            |         +----------+
                                            +---------+New header|
                                                      +----------+
```

When proposing a new header to the blockchain we are creating a new
potential fork on the blockchain. In the ideal case it will simply be
a new block on top of the _main_ current branch. We are adding blocks
after the other. In some cases it will also be a new branch, a fork.
We need to maintain some of them in order to be able to make an
informed choice when selecting the branch of consensus.

We are constructing a blockchain as we would on with git blocks:

* each block is represented by a [`Ref`];
* the [`Ref`] contains a reference to the associated `Ledger` state
  and associated `Leadership`.

A [`Branch`] contains a [`Ref`]. It allows us to follow and monitor
forks between different tasks of the blockchain module.

See Internal documentation for more details: doc/internal_design.md

[`Ref`]: ./struct.Ref.html
[`Branch`]: ./struct.Branch.html
*/
#![allow(clippy::large_enum_variant)]
use super::reference_cache::RefCache;
use crate::{
    blockcfg::{
        Block, Block0Error, BlockDate, ChainLength, Epoch, EpochRewardsInfo, Header, HeaderDesc,
        HeaderHash, Leadership, Ledger, RewardsInfoParameters,
    },
    blockchain::{Branch, Checkpoints, Multiverse, Ref, Storage, StorageError, Tip},
};
use chain_impl_mockchain::{leadership::Verification, ledger};
use chain_time::TimeFrame;
use futures::{StreamExt, TryStreamExt};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error(transparent)]
    Ledger(#[from] ledger::Error),

    #[error(transparent)]
    Block0(#[from] Block0Error),

    #[error(transparent)]
    Intercom(#[from] crate::intercom::Error),

    #[error(transparent)]
    SendError(#[from] futures::channel::mpsc::SendError),

    #[error("failed to send message to the fragments process")]
    TransactionSendError(#[from] crate::async_msg::TrySendError<crate::intercom::TransactionMsg>),

    #[error("error while creating the initial ledger out of the block0")]
    Block0InitialLedgerError(#[source] ledger::Error),

    #[error("block0 already exists in the storage")]
    Block0AlreadyInStorage,

    #[error("block0 is not yet in the storage")]
    Block0NotAlreadyInStorage,

    #[error("missing a block from the storage while recovering the blockchain: `{0}`")]
    MissingParentBlock(HeaderHash),

    #[error("tag `{0}` not found in the storage")]
    NoTag(String),

    #[error("the block header verification failed")]
    BlockHeaderVerificationFailed(#[from] HeaderChainVerifyError),

    #[error("received block `{0}` is not known from previously received headers")]
    BlockNotRequested(HeaderHash),

    #[error("block cannot be applied on top of the previous block's ledger state")]
    CannotApplyBlock(#[source] ledger::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum HeaderChainVerifyError {
    #[error(transparent)]
    NewLeadershipScheduleFailed(#[from] chain_impl_mockchain::leadership::Error),
    #[error("date is set before parent; new block: {child}, parent: {parent}")]
    BlockDateBeforeParent { child: BlockDate, parent: BlockDate },
    #[error("chain length is not incrementally increasing; new block: {child}, parent: {parent}")]
    ChainLengthNotIncremental {
        child: ChainLength,
        parent: ChainLength,
    },
}

pub type Result<T> = core::result::Result<T, Error>;

pub const MAIN_BRANCH_TAG: &str = "HEAD";

/// Performs lightweight sanity checks on information fields of a block header
/// against those in the header of the block's parent.
/// The `parent` header must have been retrieved based on, or otherwise
/// matched to, the parent block hash of `header`.
///
/// # Panics
///
/// If the parent hash in the header does not match that of the parent,
/// this function may panic.
pub fn pre_verify_link(
    header: &Header,
    parent: &Header,
) -> ::std::result::Result<(), HeaderChainVerifyError> {
    use chain_core::property::ChainLength as _;

    debug_assert_eq!(header.block_parent_hash(), parent.hash());

    if header.block_date() <= parent.block_date() {
        return Err(HeaderChainVerifyError::BlockDateBeforeParent {
            child: header.block_date(),
            parent: parent.block_date(),
        });
    }
    if header.chain_length() != parent.chain_length().next() {
        return Err(HeaderChainVerifyError::ChainLengthNotIncremental {
            child: header.chain_length(),
            parent: parent.chain_length(),
        });
    }
    Ok(())
}

/// blockchain object, can be safely shared across multiple threads. However it is better not
/// to as some operations may require a mutex.
///
/// This objects holds a reference to the persistent storage. It also holds 2 different caching
/// of objects:
///
/// * `RefCache`: a cache of blocks headers and associated states;
/// * `Multiverse`: of ledger. It is a cache of different ledger states.
///
#[derive(Clone)]
pub struct Blockchain {
    ref_cache: RefCache,

    ledgers: Multiverse<Ledger>,

    storage: Storage,

    block0: HeaderHash,

    rewards_report_all: bool,
}

pub enum PreCheckedHeader {
    /// result when the given header is already present in the
    /// local storage. The embedded `cached_reference` gives us
    /// the local cached reference is the header is already loaded
    /// in memory
    AlreadyPresent {
        /// return the Header so we can avoid doing clone
        /// of the data all the time
        header: Header,
        /// the cached reference if it was already cached.
        /// * `None` means the associated block is already in storage
        ///   but not already in the cache;
        /// * `Some(ref)` returns the local cached Ref of the block
        cached_reference: Option<Arc<Ref>>,
    },

    /// the parent is missing from the local storage
    MissingParent {
        /// return back the Header so we can avoid doing a clone
        /// of the data all the time
        header: Header,
    },

    /// The parent's is already present in the local storage and
    /// is loaded in the local cache
    HeaderWithCache {
        /// return back the Header so we can avoid doing a clone
        /// of the data all the time
        header: Header,

        /// return the locally stored parent's Ref. Already cached in memory
        /// for future processing
        parent_ref: Arc<Ref>,
    },
}

pub struct PostCheckedHeader {
    header: Header,
    epoch_leadership_schedule: Arc<Leadership>,
    parent_ledger_state: Arc<Ledger>,
    time_frame: Arc<TimeFrame>,
    previous_epoch_state: Option<Arc<Ref>>,
    epoch_rewards_info: Option<Arc<EpochRewardsInfo>>,
}

pub struct LeadershipBlock {
    pub block: Block,
    pub new_ledger: Ledger,
    pub leadership: EpochLeadership,
}

impl PostCheckedHeader {
    pub fn header(&self) -> &Header {
        &self.header
    }
}

pub enum AppliedBlock {
    New(Arc<Ref>),
    Existing(Arc<Ref>),
}

impl AppliedBlock {
    pub fn cached_ref(&self) -> Arc<Ref> {
        match self {
            AppliedBlock::New(r) => r.clone(),
            AppliedBlock::Existing(r) => r.clone(),
        }
    }

    pub fn new_ref(&self) -> Option<Arc<Ref>> {
        match self {
            AppliedBlock::New(r) => Some(r.clone()),
            AppliedBlock::Existing(_) => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum CheckHeaderProof {
    SkipFromStorage,
    Enabled,
}

impl Blockchain {
    pub fn new(
        block0: HeaderHash,
        storage: Storage,
        cache_capacity: usize,
        rewards_report_all: bool,
    ) -> Self {
        Blockchain {
            ref_cache: RefCache::new(cache_capacity),
            ledgers: Multiverse::new(),
            storage,
            block0,
            rewards_report_all,
        }
    }

    pub fn block0(&self) -> &HeaderHash {
        &self.block0
    }

    pub fn storage(&self) -> &Storage {
        &self.storage
    }

    pub async fn branches(&self) -> Result<Vec<Branch>> {
        futures::stream::iter(self.storage().get_branches()?)
            // FIXME: this should always return a valid ref, as the branches
            // are directly fetched from the node storage, but account for
            // misses in the ref cache which are not handled currently by the node
            .filter_map(|branch| async move { self.get_ref(branch).await.transpose() })
            .map(|try_ref| try_ref.map(Branch::new))
            .try_collect()
            .await
    }

    pub async fn gc(&self, tip: Arc<Ref>) -> Result<()> {
        let depth = tip.ledger().settings().epoch_stability_depth;
        self.ledgers.gc(depth).await;
        self.storage.gc(depth, tip.hash().as_ref())?;
        Ok(())
    }

    /// create and store a reference of this leader to the new
    #[allow(clippy::too_many_arguments)]
    async fn create_and_store_reference(
        &self,
        header_hash: HeaderHash,
        header: Header,
        ledger: Ledger,
        time_frame: Arc<TimeFrame>,
        leadership: Arc<Leadership>,
        epoch_rewards_info: Option<Arc<EpochRewardsInfo>>,
        previous_epoch_state: Option<Arc<Ref>>,
    ) -> Arc<Ref> {
        let chain_length = header.chain_length();

        let multiverse = self.ledgers.clone();
        let ref_cache = self.ref_cache.clone();

        let ledger_ref = multiverse.insert(chain_length, header_hash, ledger).await;
        let reference = Ref::new(
            ledger_ref,
            time_frame,
            leadership,
            epoch_rewards_info,
            header,
            previous_epoch_state,
        );
        let reference = Arc::new(reference);
        ref_cache.insert(header_hash, Arc::clone(&reference)).await;
        reference
    }

    /// get `Ref` of the given header hash
    ///
    /// once the `Ref` is in hand, it means we have the Leadership schedule associated
    /// to this block and the `Ledger` state after this block.
    ///
    /// If the future returns `None` it means we don't know about this block locally
    /// and it might be necessary to contacts the network to retrieve a missing
    /// branch
    ///
    /// TODO: the case where the block is in storage but not yet in the cache
    ///       is not implemented
    pub async fn get_ref(&self, header_hash: HeaderHash) -> Result<Option<Arc<Ref>>> {
        let maybe_ref = self.ref_cache.get(header_hash).await;
        let block_exists = self.storage.block_exists(header_hash)?;

        if maybe_ref.is_none() {
            if block_exists {
                // TODO: we have the block in the storage but it is missing
                // from the state management. Force the node to fall through
                // reloading the blocks from the storage to allow fast
                // from storage reload
            }
            Ok(None)
        } else {
            Ok(maybe_ref)
        }
    }

    /// load the header's parent `Ref`.
    async fn load_header_parent(&self, header: Header, force: bool) -> Result<PreCheckedHeader> {
        let block_id = header.hash();
        let parent_block_id = header.block_parent_hash();

        let maybe_self_ref = if force {
            Ok(None)
        } else {
            self.get_ref(block_id).await
        }?;
        let maybe_parent_ref = self.get_ref(parent_block_id).await?;

        if let Some(self_ref) = maybe_self_ref {
            Ok(PreCheckedHeader::AlreadyPresent {
                header,
                cached_reference: Some(self_ref),
            })
        } else if let Some(parent_ref) = maybe_parent_ref {
            Ok(PreCheckedHeader::HeaderWithCache { header, parent_ref })
        } else {
            Ok(PreCheckedHeader::MissingParent { header })
        }
    }

    /// load the header's parent and perform some simple verification:
    ///
    /// * check the block_date is increasing
    /// * check the chain_length is monotonically increasing
    ///
    /// At the end of this future we know either of:
    ///
    /// * the block is already present (nothing to do);
    /// * the block's parent is already present (we can then continue validation)
    /// * the block's parent is missing: we need to download it and call again
    ///   this function.
    ///
    pub async fn pre_check_header(&self, header: Header, force: bool) -> Result<PreCheckedHeader> {
        let pre_check = self.load_header_parent(header, force).await?;
        match &pre_check {
            PreCheckedHeader::HeaderWithCache { header, parent_ref } => {
                pre_verify_link(header, parent_ref.header())
                    .map(|()| pre_check)
                    .map_err(Error::BlockHeaderVerificationFailed)
            }
            _ => Ok(pre_check),
        }
    }

    /// check the header cryptographic properties and leadership's schedule
    ///
    /// on success returns the PostCheckedHeader:
    ///
    /// * the header,
    /// * the ledger state associated to the parent block
    /// * the leadership schedule associated to the header
    pub async fn post_check_header(
        &self,
        header: Header,
        parent: Arc<Ref>,
        check_header_proof: CheckHeaderProof,
    ) -> Result<PostCheckedHeader> {
        let current_date = header.block_date();
        let rewards_report_all = self.rewards_report_all;

        let EpochLeadership {
            state: parent_ledger_state,
            leadership: epoch_leadership_schedule,
            rewards_info: epoch_rewards_info,
            time_frame,
            previous_state: previous_epoch_state,
        } = new_epoch_leadership_from(current_date.epoch, parent, rewards_report_all);

        if check_header_proof == CheckHeaderProof::Enabled {
            match epoch_leadership_schedule.verify(&header) {
                Verification::Failure(error) => Err(Error::BlockHeaderVerificationFailed(
                    HeaderChainVerifyError::NewLeadershipScheduleFailed(error),
                )),
                Verification::Success => Ok(()),
            }?;
        }

        Ok(PostCheckedHeader {
            header,
            epoch_leadership_schedule,
            parent_ledger_state,
            time_frame,
            previous_epoch_state,
            epoch_rewards_info,
        })
    }

    fn apply_block_dry_run(
        &self,
        post_checked_header: &PostCheckedHeader,
        block: &Block,
    ) -> Result<Ledger> {
        let header = &post_checked_header.header;
        let block_id = header.hash();

        debug_assert!(block.header().hash() == block_id);

        let metadata = header.get_content_eval_context();

        let ledger = post_checked_header
            .parent_ledger_state
            .apply_block(block.contents(), &metadata)
            .map_err(Error::CannotApplyBlock)?;

        Ok(ledger)
    }

    fn apply_block_check_rewards(
        &self,
        post_checked_header: &PostCheckedHeader,
        ledger: &Ledger,
    ) -> Result<()> {
        // Check if rewards for this block can be distributed
        if let Some(distribution) = post_checked_header
            .epoch_leadership_schedule
            .stake_distribution()
        {
            let reward_info_dist = if self.rewards_report_all {
                RewardsInfoParameters::report_all()
            } else {
                RewardsInfoParameters::default()
            };

            ledger
                .distribute_rewards(distribution, reward_info_dist)
                .map_err(Error::CannotApplyBlock)?;
        }

        Ok(())
    }

    async fn apply_block_finalize(
        &self,
        post_checked_header: PostCheckedHeader,
        new_ledger: Ledger,
    ) -> Arc<Ref> {
        let header = post_checked_header.header;
        let block_id = header.hash();
        let epoch_leadership_schedule = post_checked_header.epoch_leadership_schedule;
        let epoch_rewards_info = post_checked_header.epoch_rewards_info;
        let time_frame = post_checked_header.time_frame;
        let previous_epoch_state = post_checked_header.previous_epoch_state;

        self.create_and_store_reference(
            block_id,
            header,
            new_ledger,
            time_frame,
            epoch_leadership_schedule,
            epoch_rewards_info,
            previous_epoch_state,
        )
        .await
    }

    async fn store_and_apply_block_finalize(
        &self,
        post_checked_header: PostCheckedHeader,
        block: Block,
        new_ledger: Ledger,
    ) -> Result<AppliedBlock> {
        let res = self.storage.put_block(&block);

        match res {
            Ok(()) | Err(StorageError::BlockAlreadyPresent) => {
                let block_ref = self
                    .apply_block_finalize(post_checked_header, new_ledger)
                    .await;

                match res {
                    Ok(()) => Ok(AppliedBlock::New(block_ref)),
                    Err(StorageError::BlockAlreadyPresent) => Ok(AppliedBlock::Existing(block_ref)),
                    _ => unreachable!(),
                }
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Apply the block on the blockchain from a post checked header
    /// and add it to the storage. If the block is already present in
    /// the storage, the returned future resolves to None. Otherwise
    /// it returns the reference to the block.
    pub async fn apply_and_store_block(
        &self,
        post_checked_header: PostCheckedHeader,
        block: Block,
    ) -> Result<AppliedBlock> {
        let new_ledger = self.apply_block_dry_run(&post_checked_header, &block)?;
        self.apply_block_check_rewards(&post_checked_header, &new_ledger)?;
        self.store_and_apply_block_finalize(post_checked_header, block, new_ledger)
            .await
    }

    /// Apply the block generated by this node. This should already have a valid state attached to
    /// it, so we do not need to verify this one.
    pub async fn apply_and_store_leadership_block(
        &self,
        leadership_block: LeadershipBlock,
    ) -> Result<AppliedBlock> {
        let LeadershipBlock {
            block,
            new_ledger,
            leadership,
        } = leadership_block;

        let header = block.header().clone();

        let EpochLeadership {
            state: parent_ledger_state,
            leadership: epoch_leadership_schedule,
            rewards_info: epoch_rewards_info,
            time_frame,
            previous_state: previous_epoch_state,
        } = leadership;

        let post_checked_header = PostCheckedHeader {
            header,
            epoch_leadership_schedule,
            parent_ledger_state,
            time_frame,
            previous_epoch_state,
            epoch_rewards_info,
        };

        self.apply_block_check_rewards(&post_checked_header, &new_ledger)?;
        self.store_and_apply_block_finalize(post_checked_header, block, new_ledger)
            .await
    }

    /// Apply the given block0 in the blockchain (updating the RefCache and the other objects)
    ///
    /// This function returns the created block0 branch. Having it will
    /// avoid searching for it in the blockchain's `branches` and perform
    /// operations to update the branch as we move along already.
    ///
    /// # Errors
    ///
    /// The resulted future may fail if
    ///
    /// * the block0 does build an invalid `Ledger`: `Error::Block0InitialLedgerError`;
    ///
    async fn apply_block0(&self, block0: &Block) -> Result<Branch> {
        let block0_id = block0.header().hash();
        let block0_date = block0.header().block_date();

        let time_frame = {
            use crate::blockcfg::Block0DataSource as _;

            let start_time = block0.start_time().map_err(Error::Block0)?;
            let slot_duration = block0.slot_duration().map_err(Error::Block0)?;

            TimeFrame::new(
                chain_time::Timeline::new(start_time),
                chain_time::SlotDuration::from_secs(slot_duration.as_secs() as u32),
            )
        };

        // we lift the creation of the ledger in the future type
        // this allow chaining of the operation and lifting the error handling
        // in the same place
        let block0_ledger = Ledger::new(block0_id, block0.contents().iter())
            .map_err(Error::Block0InitialLedgerError)?;
        let block0_leadership = Leadership::new(block0_date.epoch, &block0_ledger);

        let b = self
            .create_and_store_reference(
                block0_id,
                block0.header().clone(),
                block0_ledger,
                Arc::new(time_frame),
                Arc::new(block0_leadership),
                None, // block0 has no reward distribution
                None,
            )
            .await;
        Ok(Branch::new(b))
    }

    /// function to do the initial application of the block0 in the `Blockchain` and its
    /// storage. We assume `Block0` is not already in the `Storage`.
    ///
    /// This function returns the create block0 branch. Having it will
    /// avoid searching for it in the blockchain's `branches` and perform
    /// operations to update the branch as we move along already.
    ///
    /// # Errors
    ///
    /// The resulted future may fail if
    ///
    /// * the block0 already exists in the storage: `Error::Block0AlreadyInStorage`;
    /// * the block0 does build a valid `Ledger`: `Error::Block0InitialLedgerError`;
    /// * other errors while interacting with the storage (IO errors)
    ///
    pub async fn load_from_block0(&self, block0: Block) -> Result<Tip> {
        use chain_core::property::Block;

        let block0_id = block0.id();

        let already_exist = self.storage.block_exists(block0_id)?;

        if already_exist {
            return Err(Error::Block0AlreadyInStorage);
        }

        let block0_branch = self.apply_block0(&block0).await?;

        self.storage.put_block(&block0)?;
        self.storage.put_tag(MAIN_BRANCH_TAG, block0_id)?;
        Ok(Tip::new(block0_branch))
    }

    pub(super) async fn handle_bootstrap_block(
        &self,
        block: Block,
        check_header: CheckHeaderProof,
    ) -> Result<Arc<Ref>> {
        let header = block.header().clone();

        let pre_checked_header: PreCheckedHeader = self.pre_check_header(header, true).await?;

        let new_ref = match pre_checked_header {
            PreCheckedHeader::HeaderWithCache { header, parent_ref } => {
                let post_checked = self
                    .post_check_header(header, parent_ref, check_header)
                    .await?;

                let applied = self.apply_and_store_block(post_checked, block).await?;
                Ok(applied.cached_ref())
            }
            PreCheckedHeader::AlreadyPresent {
                cached_reference, ..
            } => Ok(cached_reference.expect("block ref was force loaded")),
            PreCheckedHeader::MissingParent { header } => {
                Err(Error::MissingParentBlock(header.block_parent_hash()))
            }
        }?;

        Ok(new_ref)
    }

    /// returns a future that will propagate the initial states and leadership
    /// from the block0 to the `Head` of the storage (the last known block which
    /// made consensus).
    ///
    /// The Future will returns a branch pointing to the `Head`.
    ///
    /// # Errors
    ///
    /// The resulted future may fail if
    ///
    /// * the block0 is not already in the storage: `Error::Block0NotAlreadyInStorage`;
    /// * the block0 does build a valid `Ledger`: `Error::Block0InitialLedgerError`;
    /// * other errors while interacting with the storage (IO errors)
    pub async fn load_from_storage(&self, block0: Block) -> Result<Tip> {
        let block0_id = block0.header().hash();
        let already_exist = self.storage.block_exists(block0_id)?;

        if !already_exist {
            return Err(Error::Block0NotAlreadyInStorage);
        }

        let opt = self.storage.get_tag(MAIN_BRANCH_TAG)?;

        let head_hash = if let Some(id) = opt {
            id
        } else {
            return Err(Error::NoTag(MAIN_BRANCH_TAG.to_owned()));
        };

        let mut last_ref = self.apply_block0(&block0).await?.get_ref();
        let mut reporter = StreamReporter::new(|stream_info| {
            let elapsed = stream_info
                .last_reported
                .elapsed()
                .expect("time went backward");
            tracing::info!(
                "loading from storage, currently at {} processing={:?} ({:?} per block) ...",
                stream_info.last_block_description.as_ref().unwrap(),
                elapsed,
                elapsed / PROCESS_LOGGING_DISTANCE as u32,
            )
        });

        let mut block_stream = self
            .storage
            .stream_from_to(block0_id, head_hash)
            .map(Box::pin)?;

        while let Some(block) = block_stream.next().await.transpose()? {
            reporter.append_block(&block);
            // this is a stream of consecutive blocks up to the last known main branch,
            // no need for special rules for updating the tip
            last_ref = self
                .handle_bootstrap_block(block, CheckHeaderProof::SkipFromStorage)
                .await?;
        }
        Ok(Tip::new(Branch::new(last_ref)))
    }

    pub fn get_checkpoints(&self, branch: &Branch) -> Checkpoints {
        Checkpoints::new_from(branch.get_ref())
    }
}

fn write_reward_info(
    epoch: Epoch,
    parent_hash: HeaderHash,
    rewards_info: &EpochRewardsInfo,
) -> std::io::Result<()> {
    use std::{
        env::var,
        fs::{rename, File},
        io::{BufWriter, Write},
        path::PathBuf,
    };

    if let Ok(directory) = var("JORMUNGANDR_REWARD_DUMP_DIRECTORY") {
        let directory = PathBuf::from(directory);

        std::fs::create_dir_all(&directory)?;

        let filepath = format!("reward-info-{}-{}", epoch, parent_hash);
        let filepath = directory.join(filepath);
        let filepath_tmp = format!("tmp.reward-info-{}-{}", epoch, parent_hash);
        let filepath_tmp = directory.join(filepath_tmp);

        {
            let file = File::create(&filepath_tmp)?;
            let mut buf = BufWriter::new(file);
            write!(&mut buf, "type,identifier,received,distributed\r\n")?;
            write!(&mut buf, "drawn,,,{}\r\n", rewards_info.drawn.0)?;
            write!(&mut buf, "fees,,,{}\r\n", rewards_info.fees.0)?;
            write!(&mut buf, "treasury,,{},\r\n", rewards_info.treasury.0)?;

            for (pool_id, (taxed, distr)) in rewards_info.stake_pools.iter() {
                write!(&mut buf, "pool,{},{},{}\r\n", pool_id, taxed.0, distr.0)?;
            }

            for (account_id, received) in rewards_info.accounts.iter() {
                write!(&mut buf, "account,{},{},\r\n", account_id, received.0)?;
            }

            buf.flush()?;
        }

        rename(filepath_tmp, filepath)?;
    }
    Ok(())
}

pub struct EpochLeadership {
    pub state: Arc<Ledger>,
    pub leadership: Arc<Leadership>,
    pub rewards_info: Option<Arc<EpochRewardsInfo>>,
    pub time_frame: Arc<TimeFrame>,
    pub previous_state: Option<Arc<Ref>>,
}

pub fn new_epoch_leadership_from(
    epoch: Epoch,
    parent: Arc<Ref>,
    rewards_report_all: bool,
) -> EpochLeadership {
    let parent_ledger_state = parent.ledger();
    let parent_epoch_leadership_schedule = parent.epoch_leadership_schedule().clone();
    let parent_epoch_rewards_info = parent.epoch_rewards_info().cloned();
    let parent_time_frame = parent.time_frame().clone();

    let parent_date = parent.block_date();

    if parent_date.epoch < epoch {
        // TODO: the time frame may change in the future, we will need to handle this
        //       special case but it is not actually clear how to modify the time frame
        //       for the blockchain
        use chain_impl_mockchain::chaintypes::ConsensusVersion;

        let ledger = parent_ledger_state
            .apply_protocol_changes()
            .expect("protocol update should not fail");

        // 1. distribute the rewards (if any) This will give us the transition state
        let (transition_state, epoch_rewards_info) =
            if let Some(distribution) = parent.epoch_leadership_schedule().stake_distribution() {
                let reward_info_dist = if rewards_report_all {
                    RewardsInfoParameters::report_all()
                } else {
                    RewardsInfoParameters::default()
                };

                let (ledger, rewards_info) = ledger
                    .distribute_rewards(distribution, reward_info_dist)
                    .expect("Distribution of rewards will not overflow");
                if let Err(err) = write_reward_info(epoch, parent.hash(), &rewards_info) {
                    panic!("Error while storing the reward dump, err {}", err)
                }
                (Arc::new(ledger), Some(Arc::new(rewards_info)))
            } else {
                (Arc::new(ledger), parent_epoch_rewards_info)
            };

        // 2. now that the rewards have been distributed, prepare the schedule
        //    for the next leader
        let epoch_state = if transition_state.consensus_version() == ConsensusVersion::GenesisPraos
        {
            // if there is no parent state available this might be because it is not
            // available in memory or it is the epoch0 or epoch1
            parent
                .last_ref_previous_epoch()
                .map(|r| r.ledger())
                .unwrap_or(parent_ledger_state)
        } else {
            transition_state.clone()
        };

        let leadership = Arc::new(Leadership::new(epoch, &epoch_state));
        let previous_epoch_state = Some(parent);
        EpochLeadership {
            state: transition_state,
            leadership,
            rewards_info: epoch_rewards_info,
            time_frame: parent_time_frame,
            previous_state: previous_epoch_state,
        }
    } else {
        EpochLeadership {
            state: parent_ledger_state,
            leadership: parent_epoch_leadership_schedule,
            rewards_info: parent_epoch_rewards_info,
            time_frame: parent_time_frame,
            previous_state: parent.last_ref_previous_epoch().map(Arc::clone),
        }
    }
}

pub struct StreamReporter<R> {
    stream_info: StreamInfo,
    report: R,
}

pub struct StreamInfo {
    pub start: std::time::SystemTime,
    pub last_reported: std::time::SystemTime,
    pub last_bytes_received: u64,
    pub bytes_received: u64,
    pub block_received: u64,
    pub last_block_description: Option<HeaderDesc>,
}

const PROCESS_LOGGING_DISTANCE: u64 = 2500;

impl<R: Fn(&StreamInfo)> StreamReporter<R> {
    pub fn new(report: R) -> Self {
        let now = std::time::SystemTime::now();
        let lbd: Option<HeaderDesc> = None;
        StreamReporter {
            stream_info: StreamInfo {
                start: now,
                last_reported: now,
                last_bytes_received: 0,
                bytes_received: 0,
                block_received: 0,
                last_block_description: lbd,
            },
            report,
        }
    }

    pub fn append_block(&mut self, b: &Block) {
        use chain_core::property::Serialize;
        self.stream_info.bytes_received += b.serialize_as_vec().unwrap().len() as u64; // TODO sad serialization back
        self.stream_info.block_received += 1;
        self.stream_info.last_block_description = Some(b.header().description());

        if self.stream_info.block_received % PROCESS_LOGGING_DISTANCE == 0 {
            (self.report)(&self.stream_info);
            self.stream_info.last_reported = std::time::SystemTime::now();
            self.stream_info.last_bytes_received = self.stream_info.bytes_received;
        }
    }
}

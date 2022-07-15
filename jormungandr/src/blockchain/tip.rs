use crate::{
    blockcfg::{FragmentId, Header, HeaderHash},
    blockchain::{
        chain_selection::{self, ComparisonResult},
        storage, Blockchain, Branch, Error, Ref, MAIN_BRANCH_TAG,
    },
    intercom::{TransactionMsg, WatchMsg},
    metrics::{Metrics, MetricsBackend},
    utils::async_msg::{self, MessageBox, MessageQueue},
};
use chain_core::property::{Block as _, Fragment as _};
use chain_impl_mockchain::block::Block;
use futures::prelude::*;
use jormungandr_lib::interfaces::FragmentStatus;
use std::{sync::Arc, time::Duration};
use tokio::{sync::RwLock, time::MissedTickBehavior};
use tracing::instrument;

// no point in updating again the tip if the old one was not processed
const INTERNAL_TIP_UPDATE_QUEUE_SIZE: usize = 1;
const BRANCH_REPROCESSING_INTERVAL: Duration = Duration::from_secs(120);

/// Handles updates to the tip.
/// Only one of this structs should be active at any given time.
#[derive(Clone)]
pub struct TipUpdater {
    tip: Tip,
    blockchain: Blockchain,
    watch_mbox: Option<MessageBox<WatchMsg>>,
    fragment_mbox: Option<MessageBox<TransactionMsg>>,
    stats_counter: Metrics,
}

impl TipUpdater {
    pub fn new(
        tip: Tip,
        blockchain: Blockchain,
        fragment_mbox: Option<MessageBox<TransactionMsg>>,
        watch_mbox: Option<MessageBox<WatchMsg>>,
        stats_counter: Metrics,
    ) -> Self {
        Self {
            tip,
            blockchain,
            fragment_mbox,
            watch_mbox,
            stats_counter,
        }
    }

    pub async fn run(&mut self, input: MessageQueue<Arc<Ref>>) {
        let mut reprocessing_interval = tokio::time::interval(BRANCH_REPROCESSING_INTERVAL);
        reprocessing_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        let (internal_mbox, internal_queue) = async_msg::channel(INTERNAL_TIP_UPDATE_QUEUE_SIZE);
        let mut stream = futures::stream::select(input, internal_queue);
        loop {
            tokio::select! {
                Some(candidate) = stream.next() => {
                    self.process_new_ref(candidate).await.unwrap_or_else(|e| tracing::error!("could not process new ref:` {}", e))
                }

                _ = reprocessing_interval.tick() => {
                    let current_tip = self.tip.get_ref().await;
                    let blockchain = self.blockchain.clone();
                    let mbox = internal_mbox.clone();
                    // Spawn this in a new task so that it does not block updates to the tip
                    tokio::spawn(Self::reprocess_tip(blockchain, current_tip, mbox));
                }

            }
        }
    }

    async fn switch_tip_branch(
        &mut self,
        candidate: Arc<Ref>,
        tip_hash: HeaderHash,
    ) -> Result<(), Error> {
        let storage = self.blockchain.storage();
        let candidate_hash = candidate.hash();
        let common_ancestor = storage.find_common_ancestor(candidate_hash, tip_hash)?;

        let stream = storage.stream_from_to(common_ancestor, candidate_hash)?;
        tokio::pin!(stream);

        // there is always at least one block in the stream
        let ancestor = stream.next().await.unwrap()?;
        if let Some(ref mut mbox) = self.fragment_mbox {
            mbox.try_send(TransactionMsg::BranchSwitch(ancestor.date().into()))?;
        }

        while let Some(block) = stream.next().await {
            let block = block?;
            let fragment_ids = block.fragments().map(|f| f.id()).collect();
            self.try_request_fragment_removal(fragment_ids, block.header())?;
        }

        self.blockchain
            .storage()
            .put_tag(MAIN_BRANCH_TAG, candidate_hash)?;

        self.tip.update_ref(candidate).await;
        Ok(())
    }

    async fn update_current_branch_tip(
        &mut self,
        candidate: Arc<Ref>,
        block: &Block,
    ) -> Result<(), Error> {
        let candidate_hash = candidate.hash();

        self.blockchain
            .storage()
            .put_tag(MAIN_BRANCH_TAG, candidate_hash)?;

        let fragment_ids = block.fragments().map(|f| f.id()).collect();
        self.try_request_fragment_removal(fragment_ids, block.header())?;

        self.tip.update_ref(candidate).await;
        Ok(())
    }

    /// process a new candidate block on top of the blockchain, this function may:
    ///
    /// * update the current tip if the candidate's parent is the current tip;
    ///
    /// If the current tip is not the one being updated we will then trigger
    /// chain selection after updating that other branch as it may be possible that
    /// this branch just became more interesting for the current consensus algorithm.
    #[instrument(level = "debug", skip(self, candidate), fields(candidate = %candidate.header().description()))]
    pub async fn process_new_ref(&mut self, candidate: Arc<Ref>) -> Result<(), Error> {
        let candidate_hash = candidate.hash();
        let storage = self.blockchain.storage();
        let tip_ref = self.tip.get_ref().await;

        match chain_selection::compare_against(storage, &tip_ref, &candidate) {
            ComparisonResult::PreferCurrent => {
                tracing::info!(
                    "rejecting candidate {} for the tip {}",
                    candidate.header().description(),
                    tip_ref.header().description(),
                );
            }
            ComparisonResult::PreferCandidate => {
                let block = storage
                    .get(candidate_hash)?
                    .ok_or(storage::Error::BlockNotFound)?;
                let tip_hash = tip_ref.hash();
                if tip_hash == candidate.block_parent_hash() {
                    tracing::info!(
                        "updating current branch tip: {} -> {}",
                        tip_ref.header().description(),
                        candidate.header().description(),
                    );
                    self.update_current_branch_tip(candidate.clone(), &block)
                        .await?;
                } else {
                    tracing::info!(
                        "switching branch from {} to {}",
                        tip_ref.header().description(),
                        candidate.header().description(),
                    );
                    self.switch_tip_branch(candidate.clone(), tip_hash).await?;
                }

                self.stats_counter.set_tip_block(&block, &candidate);

                if let Some(ref mut msg_box) = self.watch_mbox {
                    tracing::debug!("sending new tip to watch subscribers {}", candidate_hash);

                    msg_box
                        .send(WatchMsg::NewTip(candidate.header().clone()))
                        .await
                        .unwrap_or_else(|err| {
                            tracing::error!("cannot send new tip to watch client: {}", err)
                        });
                }
            }
        }

        Ok(())
    }

    fn try_request_fragment_removal(
        &mut self,
        fragment_ids: Vec<FragmentId>,
        header: &Header,
    ) -> Result<(), async_msg::TrySendError<TransactionMsg>> {
        if let Some(ref mut mbox) = self.fragment_mbox {
            let hash = header.hash().into();
            let date = header.block_date();
            let status = FragmentStatus::InABlock {
                date: date.into(),
                block: hash,
            };
            mbox.try_send(TransactionMsg::RemoveTransactions(fragment_ids, status))?;
        }

        Ok(())
    }

    /// this function will re-process the tip against the different branches.
    /// this is because a branch may have become more interesting with time
    /// moving forward and branches may have been dismissed
    #[instrument(level = "debug", skip_all, fields(current_tip = %tip.header().description()))]
    async fn reprocess_tip(
        blockchain: Blockchain,
        tip: Arc<Ref>,
        mut mbox: MessageBox<Arc<Ref>>,
    ) -> Result<(), Error> {
        use std::cmp::Ordering;
        let branches = blockchain.branches().await?;
        let storage = blockchain.storage();

        let best_branch = branches.into_iter().map(Branch::into_ref).max_by(|a, b| {
            match chain_selection::compare_against(storage, a, b) {
                ComparisonResult::PreferCurrent => Ordering::Greater,
                ComparisonResult::PreferCandidate => Ordering::Less,
            }
        });

        if let Some(new_tip) = best_branch {
            if !Arc::ptr_eq(&tip, &new_tip) {
                tracing::info!(
                    "branch reprocessing found {} as the new best tip",
                    new_tip.header().description()
                );
                mbox.try_send(new_tip).unwrap_or_else(|e| {
                    tracing::error!(
                        "{}: unable to send reprocessed tip for update, is the node overloaded?",
                        e
                    )
                });
            } else {
                tracing::debug!("reprocessing concluded, current tip is still the best branch");
            }
        } else {
            tracing::warn!("no branches found in the storage");
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Tip {
    branch: Arc<RwLock<Branch>>,
}

impl Tip {
    pub(super) fn new(branch: Branch) -> Self {
        Tip {
            branch: Arc::new(RwLock::new(branch)),
        }
    }

    pub async fn get_ref(&self) -> Arc<Ref> {
        self.branch.read().await.get_ref()
    }

    async fn update_ref(&mut self, new_ref: Arc<Ref>) {
        *self.branch.write().await = Branch::new(new_ref);
    }

    pub async fn branch(&self) -> Branch {
        (*self.branch.read().await).clone()
    }
}

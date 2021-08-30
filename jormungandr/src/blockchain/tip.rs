use crate::{
    blockcfg::{FragmentId, Header, HeaderHash},
    blockchain::{
        chain_selection::{self, ComparisonResult},
        storage, Blockchain, Branch, Error, Ref, MAIN_BRANCH_TAG,
    },
    intercom::{ExplorerMsg, TransactionMsg},
    metrics::{Metrics, MetricsBackend},
    utils::async_msg::{self, MessageBox, MessageQueue},
};
use chain_core::property::{Block as _, Fragment as _};
use chain_impl_mockchain::block::Block;
use jormungandr_lib::interfaces::FragmentStatus;
use std::sync::Arc;
use tokio::time::MissedTickBehavior;

use futures::prelude::*;

use std::time::Duration;

const BRANCH_REPROCESSING_INTERVAL: Duration = Duration::from_secs(60);

/// Handles updates to the tip.
/// Only one of this structs should be active at any given time.
#[derive(Clone)]
pub struct TipUpdater {
    tip: Tip,
    blockchain: Blockchain,
    explorer_mbox: Option<MessageBox<ExplorerMsg>>,
    fragment_mbox: Option<MessageBox<TransactionMsg>>,
    stats_counter: Metrics,
}

impl TipUpdater {
    pub fn new(
        tip: Tip,
        blockchain: Blockchain,
        fragment_mbox: Option<MessageBox<TransactionMsg>>,
        explorer_mbox: Option<MessageBox<ExplorerMsg>>,
        stats_counter: Metrics,
    ) -> Self {
        Self {
            tip,
            blockchain,
            fragment_mbox,
            explorer_mbox,
            stats_counter,
        }
    }

    pub async fn run(&mut self, mut input: MessageQueue<Arc<Ref>>) {
        let mut reprocessing_interval = tokio::time::interval(BRANCH_REPROCESSING_INTERVAL);
        reprocessing_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                Some(candidate) = input.next() => {
                    self.process_new_ref(candidate).await.unwrap_or_else(|e| tracing::error!("could not process new ref:` {}", e))
                }

                _ = reprocessing_interval.tick() => {
                    self.reprocess_tip().await.unwrap_or_else(|e| tracing::error!("could not reprocess tip:` {}", e))
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

        let branch = self
            .blockchain
            .branches_mut()
            .apply_or_create(candidate)
            .await;
        self.tip.swap(branch).await;
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
    /// * update a branch if the candidate parent is that branch's tip;
    /// * create a new branch if none of the above;
    ///
    /// If the current tip is not the one being updated we will then trigger
    /// chain selection after updating that other branch as it may be possible that
    /// this branch just became more interesting for the current consensus algorithm.
    pub async fn process_new_ref(&mut self, candidate: Arc<Ref>) -> Result<(), Error> {
        let candidate_hash = candidate.hash();
        let storage = self.blockchain.storage();
        let tip_ref = self.tip.get_ref().await;
        let block = storage
            .get(candidate_hash)?
            .ok_or(storage::Error::BlockNotFound)?;

        match chain_selection::compare_against(storage, &tip_ref, &candidate) {
            ComparisonResult::PreferCurrent => {
                tracing::info!(
                    "create new branch with tip {} | current-tip {}",
                    candidate.header().description(),
                    tip_ref.header().description(),
                );
                self.blockchain
                    .branches_mut()
                    .apply_or_create(candidate.clone())
                    .await;
            }
            ComparisonResult::PreferCandidate => {
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
                if let Some(ref mut msg_box) = self.explorer_mbox {
                    tracing::debug!("sending new tip to explorer {}", candidate_hash);
                    msg_box
                        .send(ExplorerMsg::NewTip(candidate_hash))
                        .await
                        .unwrap_or_else(|err| {
                            tracing::error!("cannot send new tip to explorer: {}", err)
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

    /// this function will re-process the tip against the different branches
    /// this is because a branch may have become more interesting with time
    /// moving forward and branches may have been dismissed
    async fn reprocess_tip(&mut self) -> Result<(), Error> {
        let branches: Vec<Arc<Ref>> = self.blockchain.branches().branches().await;
        let tip_as_ref = self.tip.get_ref().await;

        let others = branches
            .iter()
            .filter(|r| !Arc::ptr_eq(r, &tip_as_ref))
            .collect::<Vec<_>>();

        for other in others {
            self.process_new_ref(Arc::clone(other)).await?
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct Tip {
    branch: Branch,
}

impl Tip {
    // TODO: make this module private as soon as the bootstrap in refactored
    pub fn new(branch: Branch) -> Self {
        Tip { branch }
    }

    pub async fn get_ref(&self) -> Arc<Ref> {
        self.branch.get_ref().await
    }

    async fn update_ref(&mut self, new_ref: Arc<Ref>) -> Arc<Ref> {
        self.branch.update_ref(new_ref).await
    }

    async fn swap(&mut self, mut branch: Branch) {
        let mut tip_branch = self.branch.clone();
        let tr = self.branch.get_ref().await;
        let br = branch.update_ref(tr).await;
        tip_branch.update_ref(br).await;
    }

    pub fn branch(&self) -> &Branch {
        &self.branch
    }
}

use crate::{EpochInfo, EpochInfoError};
use chain_impl_mockchain::{
    block::Block,
    chaintypes::ConsensusVersion,
    header::{BlockDate, ChainLength, Epoch, Header, HeaderId},
    leadership::Leadership,
    ledger::{self, Ledger, RewardsInfoParameters},
};
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};
use thiserror::Error;

pub struct Reference {
    /// the ledger at the state of the left by applying the current block
    /// and all the previous blocks before that.
    ledger: Ledger,

    /// keeping the block's header here to save some lookup time in the storage
    /// it contains all needed to retrieve the block from the storage (the HeaderId)
    /// but also all the metadata associated to the block (parent, date, depth...).
    ///
    header: Header,

    /// the block's epoch info
    epoch_info: Arc<EpochInfo>,

    /// last `Ref`. Every time there is a transition this value will be filled with
    /// the parent `Ref`. Otherwise it will be copied from `Ref` to `Ref`.
    ///
    previous_epoch_state: Option<Arc<Reference>>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("The block could not apply successfully")]
    Ledger {
        #[source]
        source: Box<ledger::Error>,
    },

    #[error("The block's epoch validity failed to successfully apply")]
    EpochInfo {
        #[source]
        #[from]
        source: EpochInfoError,
    },

    #[error("Block's parent ({current}) does not match the block reference ({current})")]
    NotTheParentBlock {
        expected: HeaderId,
        current: HeaderId,
    },

    #[error("The block's chain length ({current}) is not the expected value ({expected})")]
    InvalidChainLength {
        expected: ChainLength,
        current: ChainLength,
    },

    #[error(
        "The block's date ({current}) is not increasing compared to the parent's block ({parent})"
    )]
    InvalidBlockDate {
        parent: BlockDate,
        current: BlockDate,
    },
}

pub enum Selection {
    PreferCurrent,
    PreferCandidate,
}

impl Reference {
    /// create a new block reference with the given block0
    ///
    /// This will mark the beginning of a new blockchain as there is no expected parents
    /// before this block. Thought he block_parent_hash may refer to a block hash from
    /// another blockchain or may have a specific meaning
    pub fn new(block0: &Block) -> Result<Self, Error> {
        let header = block0.header().clone();
        let ledger = Ledger::new(header.hash(), block0.contents().iter_slice()).map_err(|e| {
            Error::Ledger {
                source: Box::new(e),
            }
        })?;
        let epoch_info = Arc::new(EpochInfo::new(block0, &ledger)?);
        let previous_epoch_state = None;

        Ok(Self {
            ledger,
            header,
            epoch_info,
            previous_epoch_state,
        })
    }

    /// approximate a common ancestor between the given References
    ///
    /// This will lead to a common ancestor within the epoch boundary
    /// as this is the only References that may be kept.
    ///
    /// There is only 2 reasons for this function to return None:
    ///
    /// 1. the 2 blocks are from different blockchain;
    /// 2. one of the blocks are from the first epoch
    pub fn approximate_common_ancestor(self: &Arc<Self>, other: &Arc<Self>) -> Option<Arc<Self>> {
        let mut index1 = self;
        let mut index2 = other;

        if index1.hash() == index2.block_parent_hash() {
            return Some(index1.clone());
        }
        if index1.block_parent_hash() == index2.hash() {
            return Some(index2.clone());
        }
        loop {
            if index1.hash() == index2.hash() {
                return Some(index1.clone());
            }

            if index1.chain_length() < index2.chain_length() {
                if let Some(prev) = index2.previous_epoch_state.as_ref() {
                    index2 = prev;
                    continue;
                } else {
                    return None;
                }
            } else if let Some(prev) = index1.previous_epoch_state.as_ref() {
                index1 = prev;
                continue;
            } else {
                return None;
            }
        }
    }

    /// compare the current Reference with the candidate one
    ///
    pub fn select(self: &Arc<Self>, candidate: &Arc<Self>) -> Selection {
        let epoch_stability_depth = self.ledger().settings().epoch_stability_depth;

        if candidate.elapsed().is_err() {
            Selection::PreferCurrent
        } else if self.chain_length() < candidate.chain_length() {
            if let Some(common) = self.approximate_common_ancestor(candidate) {
                let common_chain_length = common.chain_length();
                if let Some(ancestor) = candidate.chain_length().nth_ancestor(epoch_stability_depth)
                {
                    if common_chain_length > ancestor {
                        Selection::PreferCurrent
                    } else {
                        Selection::PreferCandidate
                    }
                } else {
                    Selection::PreferCandidate
                }
            } else {
                Selection::PreferCurrent
            }
        } else {
            Selection::PreferCurrent
        }
    }

    /// chain a new block, expecting the new block to be a child of the given block
    ///
    /// This function will also perform all the necessary checks to make sure this
    /// block is valid within the initial context (parent hash, chain length, ledger
    /// and block signatures)
    pub fn chain(self: Arc<Self>, block: &Block) -> Result<Self, Error> {
        self.check_child(block)?;
        self.check_chain_length(block)?;
        self.check_block_date(block)?;

        let transition_state = Self::chain_epoch_info(Arc::clone(&self), block)?;
        let metadata = block.header().get_content_eval_context();

        transition_state.epoch_info.check_header(block.header())?;

        let ledger = transition_state
            .ledger()
            .apply_block(block.contents(), &metadata)
            .map_err(|e| Error::Ledger {
                source: Box::new(e),
            })?;

        Ok(Self {
            ledger,
            header: block.header().clone(),
            epoch_info: transition_state.epoch_info.clone(),
            previous_epoch_state: Some(self),
        })
    }

    /// once we suppose the end of an epoch as come, we can compute the
    /// missing steps to finalize the epoch: apply the protocol changes
    /// and distribute the rewards
    pub fn epoch_transition(&self) -> Result<Self, Error> {
        // 1. apply protocol changes
        let ledger = self
            .ledger
            .apply_protocol_changes()
            .map_err(|e| Error::Ledger {
                source: Box::new(e),
            })?;
        // 2. distribute rewards
        let ledger = if let Some(distribution) = self
            .epoch_info
            .epoch_leadership_schedule()
            .stake_distribution()
        {
            let (ledger, _rewards) = ledger
                .distribute_rewards(distribution, RewardsInfoParameters::default())
                .map_err(|e| Error::Ledger {
                    source: Box::new(e),
                })?;
            ledger
        } else {
            ledger
        };

        Ok(Self {
            ledger,
            header: self.header.clone(),
            epoch_info: self.epoch_info.clone(),
            previous_epoch_state: self.previous_epoch_state.clone(),
        })
    }

    /// compute a new epoch info from the given Reference for the given Epoch
    ///
    /// We are not performing any checks here, merely generating a new Leadership
    /// object of the given state.
    pub fn new_epoch_info(&self, epoch: Epoch) -> Result<Arc<EpochInfo>, Error> {
        // 3. prepare the leader schedule
        let leadership = if self.ledger.consensus_version() == ConsensusVersion::GenesisPraos {
            if let Some(previous_state) = self.previous_epoch_state.as_ref() {
                Leadership::new(epoch, previous_state.ledger())
            } else {
                Leadership::new(epoch, &self.ledger)
            }
        } else {
            Leadership::new(epoch, &self.ledger)
        };

        Ok(Arc::new(self.epoch_info.chain(leadership, None)))
    }

    fn chain_epoch_info(self: Arc<Self>, block: &Block) -> Result<Arc<Self>, Error> {
        let epoch = block.header().block_date().epoch;

        if self.block_date().epoch < epoch {
            let transition = self.epoch_transition()?;
            let epoch_info = transition.new_epoch_info(epoch)?;

            let previous_epoch_state = self.previous_epoch_state.clone();

            Ok(Arc::new(Self {
                ledger: transition.ledger,
                header: transition.header,
                epoch_info,
                previous_epoch_state,
            }))
        } else {
            Ok(self)
        }
    }

    fn check_child(&self, block: &Block) -> Result<(), Error> {
        if self.hash() != block.header().block_parent_hash() {
            Err(Error::NotTheParentBlock {
                expected: self.hash(),
                current: block.header().block_parent_hash(),
            })
        } else {
            Ok(())
        }
    }

    fn check_chain_length(&self, block: &Block) -> Result<(), Error> {
        if self.chain_length().increase() != block.header().chain_length() {
            Err(Error::InvalidChainLength {
                expected: self.chain_length().increase(),
                current: block.header().chain_length(),
            })
        } else {
            Ok(())
        }
    }

    fn check_block_date(&self, block: &Block) -> Result<(), Error> {
        if self.block_date() >= block.header().block_date() {
            Err(Error::InvalidBlockDate {
                parent: self.block_date(),
                current: block.header().block_date(),
            })
        } else {
            Ok(())
        }
    }

    /// retrieve the header hash of the `Ref`
    pub fn hash(&self) -> HeaderId {
        self.header.hash()
    }

    /// access the reference's parent hash
    pub fn block_parent_hash(&self) -> HeaderId {
        self.header().block_parent_hash()
    }

    /// retrieve the block date of the `Ref`
    pub fn block_date(&self) -> BlockDate {
        self.header().block_date()
    }

    /// retrieve the chain length, the number of blocks created
    /// between the block0 and this block. This is useful to compare
    /// the density of 2 branches.
    pub fn chain_length(&self) -> ChainLength {
        self.header().chain_length()
    }

    /// access the `Header` of the block pointed by this `Ref`
    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    /// retrieve the block's epoch info
    pub fn epoch_info(&self) -> Arc<EpochInfo> {
        Arc::clone(&self.epoch_info)
    }

    /// get the time the block was schedule for
    ///
    /// # panics
    ///
    /// This function will panic is the block does not coincide with the epoch's time era.
    /// This should not happen by construct as the `Reference` has been constructed and
    /// validated already.
    ///
    pub fn time(&self) -> SystemTime {
        if let Some(time) = self.epoch_info.time_of(self.header.block_date()) {
            time
        } else {
            // This error should not occur as the Reference
            // should have been constructed in the given epoch info only
            // if the block date existed in the epoch's time frame.
            panic!("The Reference has been constructed with an invalid block date on the epoch's time frame")
        }
    }

    /// retrieve the number of seconds since this block was schedule
    ///
    /// If the block was schedule in the future, the function will return
    /// an error.
    pub fn elapsed(&self) -> Result<Duration, std::time::SystemTimeError> {
        SystemTime::now().duration_since(self.time())
    }

    pub(crate) fn previous_epoch_state(&self) -> Option<&Arc<Self>> {
        self.previous_epoch_state.as_ref()
    }
}

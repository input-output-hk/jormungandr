pub mod error;
pub mod graphql;
mod indexing;
mod persistent_sequence;

use self::error::{Error, ErrorKind, Result};
use self::graphql::Context;
use self::indexing::{
    Addresses, Blocks, ChainLengths, EpochData, Epochs, ExplorerAddress, ExplorerBlock,
    ExplorerVotePlan, ExplorerVoteProposal, ExplorerVoteTally, StakePool, StakePoolBlocks,
    StakePoolData, Transactions, VotePlans,
};
use self::persistent_sequence::PersistentSequence;

use crate::blockcfg::{
    Block, ChainLength, ConfigParam, ConfigParams, ConsensusVersion, Epoch, Fragment, FragmentId,
    HeaderHash,
};
use crate::blockchain::{self, Blockchain, Multiverse, MAIN_BRANCH_TAG};
use crate::explorer::indexing::ExplorerVote;
use crate::intercom::ExplorerMsg;
use crate::utils::async_msg::MessageQueue;
use crate::utils::task::TokioServiceInfo;
use chain_addr::Discrimination;
use chain_core::property::Block as _;
use chain_impl_mockchain::certificate::{Certificate, PoolId, VotePlanId};
use chain_impl_mockchain::fee::LinearFee;
use chain_impl_mockchain::multiverse;
use futures::prelude::*;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct Explorer {
    pub db: ExplorerDB,
    pub schema: Arc<graphql::Schema>,
}

struct Branch {
    state_ref: multiverse::Ref<State>,
    length: ChainLength,
}

#[derive(Clone)]
struct Tip(Arc<RwLock<Branch>>);

#[derive(Clone)]
pub struct ExplorerDB {
    /// Structure that keeps all the known states to allow easy branch management
    /// each new block is indexed by getting its previous `State` from the multiverse
    /// and inserted a new updated one.
    multiverse: Multiverse<State>,
    /// This keeps track of the longest chain seen until now. All the queries are
    /// performed using the state of this branch, the HeaderHash is used as key for the
    /// multiverse, and the ChainLength is used in the updating process.
    longest_chain_tip: Tip,
    pub blockchain_config: BlockchainConfig,
    blockchain: Blockchain,
    blockchain_tip: blockchain::Tip,
}

#[derive(Clone)]
pub struct BlockchainConfig {
    /// Used to construct `Address` from `AccountIndentifier` when processing transaction
    /// inputs
    discrimination: Discrimination,
    consensus_version: ConsensusVersion,
    fees: LinearFee,
}

/// Inmutable data structure used to represent the explorer's state at a given Block
/// A new state can be obtained to from a Block and it's previous state, getting two
/// independent states but with memory sharing to minimize resource utilization
#[derive(Clone)]
struct State {
    parent_ref: Option<multiverse::Ref<State>>,
    transactions: Transactions,
    blocks: Blocks,
    addresses: Addresses,
    epochs: Epochs,
    chain_lengths: ChainLengths,
    stake_pool_data: StakePool,
    stake_pool_blocks: StakePoolBlocks,
    vote_plans: VotePlans,
}

#[derive(Clone)]
pub struct Settings {
    /// This is the prefix that's used for the Address bech32 string representation in the
    /// responses (in the queries any prefix can be used). base32 serialization could
    /// also be used, but the `Address` struct doesn't have a deserialization method right
    /// now
    pub address_bech32_prefix: String,
}

impl Explorer {
    pub fn new(db: ExplorerDB, schema: graphql::Schema) -> Explorer {
        Explorer {
            db,
            schema: Arc::new(schema),
        }
    }

    pub fn context(&self) -> Context {
        Context {
            db: self.db.clone(),
            settings: Settings {
                // Hardcoded bech32 prefix
                address_bech32_prefix: "addr".to_owned(),
            },
        }
    }

    pub async fn start(&mut self, info: TokioServiceInfo, messages: MessageQueue<ExplorerMsg>) {
        messages
            .for_each(|input| async {
                match input {
                    ExplorerMsg::NewBlock(block) => {
                        let mut explorer_db = self.db.clone();
                        let logger = info.logger().clone();
                        info.spawn_fallible("apply block", async move {
                            explorer_db
                                .apply_block(block)
                                .map(move |result| match result {
                                    // XXX: There is no garbage collection now, so the GCRoot is not used
                                    Ok(_gc_root) => Ok(()),
                                    Err(err) => {
                                        error!(logger, "Explorer error: {}", err);
                                        Err(())
                                    }
                                })
                                .await
                        });
                    }
                }
            })
            .await;
    }
}

impl ExplorerDB {
    /// Apply all the blocks in the [block0, MAIN_BRANCH_TAG], also extract the static
    /// Blockchain settings from the Block0 (Discrimination)
    /// This function is only called once on the node's bootstrap phase
    pub async fn bootstrap(
        block0: Block,
        blockchain: &Blockchain,
        blockchain_tip: blockchain::Tip,
    ) -> Result<Self> {
        let blockchain_config = BlockchainConfig::from_config_params(
            block0
                .contents
                .iter()
                .filter_map(|fragment| match fragment {
                    Fragment::Initial(config_params) => Some(config_params),
                    _ => None,
                })
                .next()
                .expect("the Initial fragment to be present in the genesis block"),
        );

        let block = ExplorerBlock::resolve_from(
            &block0,
            indexing::ExplorerBlockBuildingContext {
                discrimination: blockchain_config.discrimination,
                prev_transactions: &Transactions::new(),
                prev_blocks: &Blocks::new(),
            },
        );

        let blocks = apply_block_to_blocks(Blocks::new(), &block)?;
        let epochs = apply_block_to_epochs(Epochs::new(), &block);
        let chain_lengths = apply_block_to_chain_lengths(ChainLengths::new(), &block)?;
        let transactions = apply_block_to_transactions(Transactions::new(), &block)?;
        let addresses = apply_block_to_addresses(Addresses::new(), &block)?;
        let (stake_pool_data, stake_pool_blocks) =
            apply_block_to_stake_pools(StakePool::new(), StakePoolBlocks::new(), &block);
        let vote_plans = apply_block_to_vote_plans(VotePlans::new(), &blockchain_tip, &block);

        let initial_state = State {
            blocks,
            epochs,
            chain_lengths,
            transactions,
            addresses,
            stake_pool_data,
            stake_pool_blocks,
            parent_ref: None,
            vote_plans,
        };

        let multiverse = Multiverse::<State>::new();
        let block0_id = block0.id();
        let initial_state_ref = multiverse
            .insert(block0.chain_length(), block0_id, initial_state)
            .await;

        let bootstraped_db = ExplorerDB {
            multiverse,
            longest_chain_tip: Tip::new(Branch {
                state_ref: initial_state_ref,
                length: block0.header.chain_length(),
            }),
            blockchain_config,
            blockchain: blockchain.clone(),
            blockchain_tip,
        };

        let maybe_head = blockchain.storage().get_tag(MAIN_BRANCH_TAG)?;
        let stream = match maybe_head {
            Some(head) => blockchain.storage().stream_from_to(block0_id, head)?,
            None => {
                return Err(Error::from(ErrorKind::BootstrapError(
                    "Couldn't read the HEAD tag from storage".to_owned(),
                )))
            }
        };

        let mut db = stream
            .map_err(Error::from)
            .try_fold(bootstraped_db, |mut db, block| async move {
                db.apply_block(block).await?;
                Ok(db)
            })
            .await?;

        for branch in blockchain.branches().branches().await.iter() {
            let mut hash = branch.hash();
            let mut blocks = vec![];
            loop {
                if db.get_block(&hash).await.is_some() {
                    break;
                }
                let block = blockchain.storage().get(hash)?.ok_or_else(|| {
                    Error::from(ErrorKind::BootstrapError(format!(
                        "couldn't get block {} from the storage",
                        hash
                    )))
                })?;
                hash = block.header.block_parent_hash();
                blocks.push(block);
            }
            while let Some(block) = blocks.pop() {
                db.apply_block(block).await?;
            }
        }

        Ok(db)
    }

    /// Try to add a new block to the indexes, this can fail if the parent of the block is
    /// not processed. Also, update the longest seen chain with this block as tip if its
    /// chain length is greater than the current.
    /// This doesn't perform any validation on the given block and the previous state, it
    /// is assumed that the Block is valid
    async fn apply_block(&mut self, block: Block) -> Result<multiverse::Ref<State>> {
        let previous_block = block.header.block_parent_hash();
        let chain_length = block.header.chain_length();
        let block_id = block.header.hash();
        let multiverse = self.multiverse.clone();
        let current_tip = self.longest_chain_tip.clone();
        let discrimination = self.blockchain_config.discrimination;

        let previous_state = multiverse
            .get_ref(previous_block)
            .await
            .ok_or_else(|| Error::from(ErrorKind::AncestorNotFound(format!("{}", block.id()))))?;
        let State {
            parent_ref: _,
            transactions,
            blocks,
            addresses,
            epochs,
            chain_lengths,
            stake_pool_data,
            stake_pool_blocks,
            vote_plans,
        } = previous_state.state().clone();

        let explorer_block = ExplorerBlock::resolve_from(
            &block,
            indexing::ExplorerBlockBuildingContext {
                discrimination,
                prev_transactions: &transactions,
                prev_blocks: &blocks,
            },
        );
        let (stake_pool_data, stake_pool_blocks) =
            apply_block_to_stake_pools(stake_pool_data, stake_pool_blocks, &explorer_block);

        let state_ref = multiverse
            .insert(
                chain_length,
                block_id,
                State {
                    parent_ref: Some(previous_state),
                    transactions: apply_block_to_transactions(transactions, &explorer_block)?,
                    blocks: apply_block_to_blocks(blocks, &explorer_block)?,
                    addresses: apply_block_to_addresses(addresses, &explorer_block)?,
                    epochs: apply_block_to_epochs(epochs, &explorer_block),
                    chain_lengths: apply_block_to_chain_lengths(chain_lengths, &explorer_block)?,
                    stake_pool_data,
                    stake_pool_blocks,
                    vote_plans: apply_block_to_vote_plans(
                        vote_plans,
                        &self.blockchain_tip,
                        &explorer_block,
                    ),
                },
            )
            .await;

        current_tip
            .compare_and_replace(Branch {
                state_ref: state_ref.clone(),
                length: chain_length,
            })
            .await;

        Ok(state_ref)
    }

    pub async fn get_latest_block_hash(&self) -> HeaderHash {
        self.longest_chain_tip.get_block_id().await
    }

    pub async fn get_block(&self, block_id: &HeaderHash) -> Option<ExplorerBlock> {
        let block_id = *block_id;
        self.with_latest_state(move |state| {
            state.blocks.lookup(&block_id).map(|b| b.as_ref().clone())
        })
        .await
    }

    pub async fn get_epoch(&self, epoch: Epoch) -> Option<EpochData> {
        self.with_latest_state(move |state| state.epochs.lookup(&epoch).map(|e| e.as_ref().clone()))
            .await
    }

    pub async fn find_block_by_chain_length(
        &self,
        chain_length: ChainLength,
    ) -> Option<HeaderHash> {
        self.with_latest_state(move |state| {
            state
                .chain_lengths
                .lookup(&chain_length)
                .map(|b| *b.as_ref())
        })
        .await
    }

    pub async fn find_block_hash_by_transaction(
        &self,
        transaction_id: &FragmentId,
    ) -> Option<HeaderHash> {
        self.with_latest_state(move |state| {
            state
                .transactions
                .lookup(&transaction_id)
                .map(|id| *id.as_ref())
        })
        .await
    }

    pub async fn get_transactions_by_address(
        &self,
        address: &ExplorerAddress,
    ) -> Option<PersistentSequence<FragmentId>> {
        let address = address.clone();
        self.with_latest_state(move |state| {
            state
                .addresses
                .lookup(&address)
                .map(|set| set.as_ref().clone())
        })
        .await
    }

    // Get the hashes of all blocks in the range [from, to)
    // the ChainLength is returned to for easy of use in the case where
    // `to` is greater than the max
    pub async fn get_block_hash_range(
        &self,
        from: ChainLength,
        to: ChainLength,
    ) -> Vec<(HeaderHash, ChainLength)> {
        let from = u32::from(from);
        let to = u32::from(to);

        self.with_latest_state(move |state| {
            (from..to)
                .filter_map(|i| {
                    state
                        .chain_lengths
                        .lookup(&i.into())
                        .map(|b| (*b.as_ref(), i.into()))
                })
                .collect()
        })
        .await
    }

    pub async fn get_stake_pool_blocks(
        &self,
        pool: &PoolId,
    ) -> Option<PersistentSequence<HeaderHash>> {
        let pool = pool.clone();
        self.with_latest_state(move |state| {
            state
                .stake_pool_blocks
                .lookup(&pool)
                .map(|i| i.as_ref().clone())
        })
        .await
    }

    pub async fn get_stake_pool_data(&self, pool: &PoolId) -> Option<StakePoolData> {
        let pool = pool.clone();
        self.with_latest_state(move |state| {
            state
                .stake_pool_data
                .lookup(&pool)
                .map(|i| i.as_ref().clone())
        })
        .await
    }

    pub async fn get_stake_pools(&self) -> Vec<(PoolId, Arc<StakePoolData>)> {
        self.with_latest_state(move |state| {
            state
                .stake_pool_data
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .await
    }

    pub async fn get_vote_plan_by_id(&self, vote_plan_id: &VotePlanId) -> Option<ExplorerVotePlan> {
        self.with_latest_state(move |state| {
            state
                .vote_plans
                .lookup(vote_plan_id)
                .map(|vote_plan| vote_plan.as_ref().clone())
        })
        .await
    }

    pub async fn get_vote_plans(&self) -> Vec<(VotePlanId, Arc<ExplorerVotePlan>)> {
        self.with_latest_state(move |state| {
            state
                .vote_plans
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .await
    }

    /// run given function with the longest branch's state
    async fn with_latest_state<T>(&self, f: impl Fn(State) -> T) -> T {
        let multiverse = self.multiverse.clone();
        let branch_id = self.get_latest_block_hash().await;
        let maybe_state = multiverse.get(branch_id).await;
        let state = maybe_state.expect("the longest chain to be indexed");
        f(state)
    }

    fn blockchain(&self) -> &Blockchain {
        &self.blockchain
    }
}

fn apply_block_to_transactions(
    mut transactions: Transactions,
    block: &ExplorerBlock,
) -> Result<Transactions> {
    let block_id = block.id();
    let ids = block.transactions.values().map(|tx| tx.id());

    for id in ids {
        transactions = transactions
            .insert(id, Arc::new(block_id))
            .map_err(|_| ErrorKind::TransactionAlreadyExists(format!("{}", id)))?;
    }

    Ok(transactions)
}

fn apply_block_to_blocks(blocks: Blocks, block: &ExplorerBlock) -> Result<Blocks> {
    let block_id = block.id();
    blocks
        .insert(block_id, Arc::new(block.clone()))
        .map_err(|_| Error::from(ErrorKind::BlockAlreadyExists(format!("{}", block_id))))
}

fn apply_block_to_addresses(mut addresses: Addresses, block: &ExplorerBlock) -> Result<Addresses> {
    let transactions = block.transactions.values();

    for tx in transactions {
        let id = tx.id();

        // A Hashset is used for preventing duplicates when the address is both an
        // input and an output in the given transaction

        let included_addresses: std::collections::HashSet<ExplorerAddress> = tx
            .outputs()
            .iter()
            .map(|output| output.address.clone())
            .chain(tx.inputs().iter().map(|input| input.address.clone()))
            .collect();

        for address in included_addresses {
            addresses = addresses.insert_or_update_simple(
                address,
                Arc::new(PersistentSequence::new().append(id)),
                |set| {
                    let new_set = set.append(id);
                    Some(Arc::new(new_set))
                },
            )
        }
    }
    Ok(addresses)
}

fn apply_block_to_epochs(epochs: Epochs, block: &ExplorerBlock) -> Epochs {
    let epoch_id = block.date().epoch;
    let block_id = block.id();

    epochs.insert_or_update_simple(
        epoch_id,
        Arc::new(EpochData {
            first_block: block_id,
            last_block: block_id,
            total_blocks: 0,
        }),
        |data| {
            Some(Arc::new(EpochData {
                first_block: data.first_block,
                last_block: block_id,
                total_blocks: data.total_blocks + 1,
            }))
        },
    )
}

fn apply_block_to_chain_lengths(
    chain_lengths: ChainLengths,
    block: &ExplorerBlock,
) -> Result<ChainLengths> {
    let new_block_chain_length = block.chain_length();
    let new_block_hash = block.id();
    chain_lengths
        .insert(new_block_chain_length, Arc::new(new_block_hash))
        .map_err(|_| {
            // I think this shouldn't happen
            Error::from(ErrorKind::ChainLengthBlockAlreadyExists(u32::from(
                new_block_chain_length,
            )))
        })
}

fn apply_block_to_stake_pools(
    data: StakePool,
    blocks: StakePoolBlocks,
    block: &ExplorerBlock,
) -> (StakePool, StakePoolBlocks) {
    let mut blocks = match &block.producer() {
        indexing::BlockProducer::StakePool(id) => blocks
            .update(
                &id,
                |array: &Arc<PersistentSequence<HeaderHash>>| -> std::result::Result<_, Infallible> {
                    Ok(Some(Arc::new(array.append(block.id()))))
                },
            )
            .expect("block to be created by registered stake pool"),
        indexing::BlockProducer::BftLeader(_) => blocks,
        indexing::BlockProducer::None => blocks,
    };

    let mut data = data;

    for tx in block.transactions.values() {
        if let Some(cert) = &tx.certificate {
            blocks = match cert {
                Certificate::PoolRegistration(registration) => blocks
                    .insert(registration.to_id(), Arc::new(PersistentSequence::new()))
                    .expect("pool was registered more than once"),
                _ => blocks,
            };
            data = match cert {
                Certificate::PoolRegistration(registration) => data
                    .insert(
                        registration.to_id(),
                        Arc::new(StakePoolData {
                            registration: registration.clone(),
                            retirement: None,
                        }),
                    )
                    .expect("pool was registered more than once"),
                Certificate::PoolRetirement(retirement) => {
                    data.update::<_, Infallible>(&retirement.pool_id, |pool_data| {
                        Ok(Some(Arc::new(StakePoolData {
                            registration: pool_data.registration.clone(),
                            retirement: Some(retirement.clone()),
                        })))
                    })
                    .expect("pool was retired before registered");
                    data
                }
                _ => data,
            };
        }
    }

    (data, blocks)
}

fn apply_block_to_vote_plans(
    mut vote_plans: VotePlans,
    blockchain_tip: &blockchain::Tip,
    block: &ExplorerBlock,
) -> VotePlans {
    for tx in block.transactions.values() {
        if let Some(cert) = &tx.certificate {
            vote_plans = match cert {
                Certificate::VotePlan(vote_plan) => vote_plans
                    .insert(
                        vote_plan.to_id(),
                        Arc::new(ExplorerVotePlan {
                            id: vote_plan.to_id(),
                            vote_start: vote_plan.vote_start(),
                            vote_end: vote_plan.vote_end(),
                            committee_end: vote_plan.committee_end(),
                            payload_type: vote_plan.payload_type(),
                            proposals: vote_plan
                                .proposals()
                                .iter()
                                .map(|proposal| ExplorerVoteProposal {
                                    proposal_id: proposal.external_id().clone(),
                                    options: proposal.options().clone(),
                                    tally: None,
                                    votes: Default::default(),
                                })
                                .collect(),
                        }),
                    )
                    .unwrap(),
                Certificate::VoteCast(vote_cast) => {
                    use chain_impl_mockchain::vote::Payload;
                    let voter = tx.inputs[0].address.clone();
                    match vote_cast.payload() {
                        Payload::Public { choice } => vote_plans
                            .update(vote_cast.vote_plan(), |vote_plan| {
                                let mut proposals = vote_plan.proposals.clone();
                                proposals[vote_cast.proposal_index() as usize].votes = proposals
                                    [vote_cast.proposal_index() as usize]
                                    .votes
                                    .insert_or_update(
                                        voter,
                                        Arc::new(ExplorerVote::Public(*choice)),
                                        |_| {
                                            Ok::<_, std::convert::Infallible>(Some(Arc::new(
                                                ExplorerVote::Public(*choice),
                                            )))
                                        },
                                    )
                                    .unwrap();
                                let vote_plan = ExplorerVotePlan {
                                    proposals,
                                    ..(**vote_plan).clone()
                                };
                                Ok::<_, std::convert::Infallible>(Some(Arc::new(vote_plan)))
                            })
                            .unwrap(),
                        Payload::Private {
                            proof,
                            encrypted_vote,
                        } => vote_plans
                            .update(vote_cast.vote_plan(), |vote_plan| {
                                let mut proposals = vote_plan.proposals.clone();
                                proposals[vote_cast.proposal_index() as usize].votes = proposals
                                    [vote_cast.proposal_index() as usize]
                                    .votes
                                    .insert_or_update(
                                        voter,
                                        Arc::new(ExplorerVote::Private {
                                            proof: proof.clone(),
                                            encrypted_vote: encrypted_vote.clone(),
                                        }),
                                        |_| {
                                            Ok::<_, std::convert::Infallible>(Some(Arc::new(
                                                ExplorerVote::Private {
                                                    proof: proof.clone(),
                                                    encrypted_vote: encrypted_vote.clone(),
                                                },
                                            )))
                                        },
                                    )
                                    .unwrap();
                                let vote_plan = ExplorerVotePlan {
                                    proposals,
                                    ..(**vote_plan).clone()
                                };
                                Ok::<_, std::convert::Infallible>(Some(Arc::new(vote_plan)))
                            })
                            .unwrap(),
                    }
                }
                Certificate::VoteTally(vote_tally) => {
                    use chain_impl_mockchain::vote::PayloadType;
                    vote_plans
                        .update(vote_tally.id(), |vote_plan| {
                            let proposals_from_state =
                                futures::executor::block_on(blockchain_tip.get_ref())
                                    .active_vote_plans()
                                    .into_iter()
                                    .find_map(|vps| {
                                        if vps.id != vote_plan.id {
                                            return None;
                                        }
                                        Some(vps.proposals)
                                    })
                                    .unwrap();
                            let proposals = vote_plan
                                .proposals
                                .clone()
                                .into_iter()
                                .enumerate()
                                .map(|(index, mut proposal)| {
                                    proposal.tally = Some(match vote_tally.tally_type() {
                                        PayloadType::Public => ExplorerVoteTally::Public {
                                            results: proposals_from_state[index]
                                                .tally
                                                .clone()
                                                .unwrap()
                                                .result()
                                                .unwrap()
                                                .results()
                                                .to_vec(),
                                            options: proposal.options.clone(),
                                        },
                                        PayloadType::Private => ExplorerVoteTally::Private {
                                            results: proposals_from_state[index]
                                                .tally
                                                .clone()
                                                .unwrap()
                                                .result()
                                                .map(|tally_results| {
                                                    tally_results.results().to_vec()
                                                }),
                                            options: proposal.options.clone(),
                                        },
                                    });
                                    proposal
                                })
                                .collect();
                            let vote_plan = ExplorerVotePlan {
                                proposals,
                                ..(**vote_plan).clone()
                            };
                            Ok::<_, std::convert::Infallible>(Some(Arc::new(vote_plan)))
                        })
                        .unwrap()
                }
                _ => vote_plans,
            }
        }
    }

    vote_plans
}

impl BlockchainConfig {
    fn from_config_params(params: &ConfigParams) -> BlockchainConfig {
        let discrimination = params
            .iter()
            .filter_map(|param| match param {
                ConfigParam::Discrimination(discrimination) => Some(discrimination),
                _ => None,
            })
            .next()
            .expect("the discrimination to be present");

        let consensus_version = params
            .iter()
            .filter_map(|param| match param {
                ConfigParam::ConsensusVersion(version) => Some(version),
                _ => None,
            })
            .next()
            .expect("consensus version to be present");

        let fees = params
            .iter()
            .filter_map(|param| match param {
                ConfigParam::LinearFee(fee) => Some(fee),
                _ => None,
            })
            .next()
            .expect("fee is not in config params");

        BlockchainConfig {
            discrimination: *discrimination,
            consensus_version: *consensus_version,
            fees: *fees,
        }
    }
}

impl Tip {
    fn new(branch: Branch) -> Tip {
        Tip(Arc::new(RwLock::new(branch)))
    }

    async fn compare_and_replace(&self, other: Branch) {
        let mut current = self.0.write().await;

        if other.length > (*current).length {
            *current = Branch {
                state_ref: other.state_ref,
                length: other.length,
            };
        }
    }

    async fn get_block_id(&self) -> HeaderHash {
        let guard = self.0.read().await;
        *guard.state_ref.id()
    }
}

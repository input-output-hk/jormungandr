mod certificates;
mod config_param;
mod connections;
mod error;
mod scalars;

use self::{
    config_param::{EpochStabilityDepth, LinearFee},
    connections::{
        compute_interval, ConnectionFields, InclusivePaginationInterval, PaginationInterval,
        ValidatedPaginationArguments,
    },
    error::ApiError,
    scalars::{
        BlockCount, ChainLength, EpochNumber, ExternalProposalId, IndexCursor, NonZero,
        PayloadType, PoolCount, PoolId, PublicKey, Slot, TransactionCount, Value, VoteOptionRange,
        VotePlanId, VotePlanStatusCount, Weight,
    },
};
use crate::db::{
    indexing::{
        BlockProducer, EpochData, ExplorerAddress, ExplorerBlock, ExplorerTransaction,
        ExplorerVote, ExplorerVotePlan, ExplorerVoteTally, StakePoolData,
    },
    persistent_sequence::PersistentSequence,
    ExplorerDb, Settings as ChainSettings,
};
use async_graphql::{
    connection::{query, Connection, Edge, EmptyFields},
    Context, EmptyMutation, FieldError, FieldResult, Object, SimpleObject, Subscription, Union,
};
use cardano_legacy_address::Addr as OldAddress;
use certificates::*;
use chain_impl_mockchain::{
    block::{BlockDate as InternalBlockDate, Epoch as InternalEpoch, HeaderId as HeaderHash},
    certificate,
    fragment::FragmentId,
    key::BftLeaderId,
    vote::{EncryptedVote, ProofOfCorrectVote},
};
use std::{
    convert::{TryFrom, TryInto},
    str::FromStr,
    sync::Arc,
};

pub struct Branch {
    state: crate::db::Ref,
    id: HeaderHash,
}

impl Branch {
    async fn try_from_id(id: HeaderHash, context: &EContext) -> FieldResult<Branch> {
        context
            .db
            .get_branch(&id)
            .await
            .map(|state| Branch { state, id })
            .ok_or_else(|| ApiError::NotFound("branch not found".to_string()).into())
    }

    fn from_id_and_state(id: HeaderHash, state: crate::db::Ref) -> Branch {
        Branch { state, id }
    }
}

#[Object]
impl Branch {
    pub async fn id(&self) -> String {
        format!("{}", self.id)
    }

    pub async fn block(&self) -> Block {
        Block::from_contents(Arc::clone(
            self.state.state().blocks.lookup(&self.id).unwrap(),
        ))
    }

    pub async fn blocks(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<String>,
        after: Option<String>,
    ) -> FieldResult<Connection<IndexCursor, Block, ConnectionFields<BlockCount>, EmptyFields>>
    {
        let block0 = 0u32;
        let chain_length = self.state.state().blocks.size();

        query(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let boundaries = PaginationInterval::Inclusive(InclusivePaginationInterval {
                    lower_bound: block0,
                    // this try_from cannot fail, as there can't be more than 2^32
                    // blocks (because ChainLength is u32)
                    upper_bound: u32::try_from(chain_length).unwrap(),
                });

                let pagination_arguments = ValidatedPaginationArguments {
                    first,
                    last,
                    before: before.map(u32::try_from).transpose()?,
                    after: after.map(u32::try_from).transpose()?,
                };

                let (range, page_meta) = compute_interval(boundaries, pagination_arguments)?;

                let mut connection = Connection::with_additional_fields(
                    page_meta.has_previous_page,
                    page_meta.has_next_page,
                    ConnectionFields {
                        total_count: page_meta.total_count,
                    },
                );

                let edges = match range {
                    PaginationInterval::Empty => Default::default(),
                    PaginationInterval::Inclusive(range) => {
                        let a = range.lower_bound.into();
                        let b = range.upper_bound.checked_add(1).unwrap().into();
                        self.state.state().get_block_hash_range(a, b)
                    }
                };

                connection
                    .edges
                    .extend(edges.iter().map(|(h, chain_length)| {
                        Edge::new(
                            IndexCursor::from(u32::from(*chain_length)),
                            Block::from_valid_hash(*h),
                        )
                    }));

                Ok::<_, async_graphql::Error>(connection)
            },
        )
        .await
    }

    async fn transactions_by_address(
        &self,
        address_bech32: String,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<String>,
        after: Option<String>,
    ) -> FieldResult<
        Connection<IndexCursor, Transaction, ConnectionFields<TransactionCount>, EmptyFields>,
    > {
        let address = chain_addr::AddressReadable::from_string_anyprefix(&address_bech32)
            .map(|adr| ExplorerAddress::New(adr.to_address()))
            .or_else(|_| OldAddress::from_str(&address_bech32).map(ExplorerAddress::Old))
            .map_err(|_| ApiError::InvalidAddress(address_bech32.to_string()))?;

        let transactions = self
            .state
            .state()
            .transactions_by_address(&address)
            .unwrap_or_else(PersistentSequence::<FragmentId>::new);

        let len = transactions.len();

        query(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let boundaries = if len > 0 {
                    PaginationInterval::Inclusive(InclusivePaginationInterval {
                        lower_bound: 0u64,
                        upper_bound: len,
                    })
                } else {
                    PaginationInterval::Empty
                };

                let pagination_arguments = ValidatedPaginationArguments {
                    first,
                    last,
                    before: before.map(TryInto::try_into).transpose()?,
                    after: after.map(TryInto::try_into).transpose()?,
                };

                let (range, page_meta) = compute_interval(boundaries, pagination_arguments)?;

                let mut connection = Connection::with_additional_fields(
                    page_meta.has_previous_page,
                    page_meta.has_next_page,
                    ConnectionFields {
                        total_count: page_meta.total_count,
                    },
                );

                let edges = match range {
                    PaginationInterval::Empty => vec![],
                    PaginationInterval::Inclusive(range) => (range.lower_bound..=range.upper_bound)
                        .filter_map(|i| transactions.get(i).map(|h| (HeaderHash::clone(h), i)))
                        .collect(),
                };

                connection.edges.extend(edges.iter().map(|(h, i)| {
                    Edge::new(IndexCursor::from(*i), Transaction::from_valid_id(*h))
                }));

                Ok::<_, async_graphql::Error>(connection)
            },
        )
        .await
    }

    pub async fn all_vote_plans(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<String>,
        after: Option<String>,
    ) -> FieldResult<
        Connection<IndexCursor, VotePlanStatus, ConnectionFields<VotePlanStatusCount>, EmptyFields>,
    > {
        let mut vote_plans = self.state.state().get_vote_plans();

        vote_plans.sort_unstable_by_key(|(id, _data)| id.clone());

        query(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let boundaries = if !vote_plans.is_empty() {
                    PaginationInterval::Inclusive(InclusivePaginationInterval {
                        lower_bound: 0u32,
                        upper_bound: vote_plans
                            .len()
                            .checked_sub(1)
                            .unwrap()
                            .try_into()
                            .expect("tried to paginate more than 2^32 elements"),
                    })
                } else {
                    PaginationInterval::Empty
                };

                let pagination_arguments = ValidatedPaginationArguments {
                    first,
                    last,
                    before: before.map(u32::try_from).transpose()?,
                    after: after.map(u32::try_from).transpose()?,
                };

                let (range, page_meta) = compute_interval(boundaries, pagination_arguments)?;
                let mut connection = Connection::with_additional_fields(
                    page_meta.has_previous_page,
                    page_meta.has_next_page,
                    ConnectionFields {
                        total_count: page_meta.total_count,
                    },
                );

                let edges = match range {
                    PaginationInterval::Empty => vec![],
                    PaginationInterval::Inclusive(range) => {
                        let from = range.lower_bound;
                        let to = range.upper_bound;

                        (from..=to)
                            .map(|i: u32| {
                                let (_pool_id, vote_plan_data) =
                                    &vote_plans[usize::try_from(i).unwrap()];
                                (
                                    VotePlanStatus::vote_plan_from_data(Arc::clone(vote_plan_data)),
                                    i,
                                )
                            })
                            .collect::<Vec<(VotePlanStatus, u32)>>()
                    }
                };

                connection.edges.extend(
                    edges
                        .iter()
                        .map(|(vps, cursor)| Edge::new(IndexCursor::from(*cursor), vps.clone())),
                );

                Ok::<_, async_graphql::Error>(connection)
            },
        )
        .await
    }

    pub async fn all_stake_pools(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<String>,
        after: Option<String>,
    ) -> FieldResult<Connection<IndexCursor, Pool, ConnectionFields<PoolCount>, EmptyFields>> {
        let mut stake_pools = self.state.state().get_stake_pools();

        // Although it's probably not a big performance concern
        // There are a few alternatives to not have to sort this
        // - A separate data structure can be used to track InsertionOrder -> PoolId
        // (or any other order)
        // - Find some way to rely in the Hamt iterator order (but I think this is probably not a good idea)
        stake_pools.sort_unstable_by_key(|(id, _data)| id.clone());

        query(
            after.map(Into::into),
            before.map(Into::into),
            first,
            last,
            |after, before, first, last| async move {
                let boundaries = if !stake_pools.is_empty() {
                    PaginationInterval::Inclusive(InclusivePaginationInterval {
                        lower_bound: 0u32,
                        upper_bound: stake_pools
                            .len()
                            .checked_sub(1)
                            .unwrap()
                            .try_into()
                            .expect("tried to paginate more than 2^32 elements"),
                    })
                } else {
                    PaginationInterval::Empty
                };

                let pagination_arguments = ValidatedPaginationArguments {
                    first,
                    last,
                    before: before.map(u32::try_from).transpose()?,
                    after: after.map(u32::try_from).transpose()?,
                };

                let (range, page_meta) = compute_interval(boundaries, pagination_arguments)?;
                let mut connection = Connection::with_additional_fields(
                    page_meta.has_previous_page,
                    page_meta.has_next_page,
                    ConnectionFields {
                        total_count: page_meta.total_count,
                    },
                );

                let edges = match range {
                    PaginationInterval::Empty => vec![],
                    PaginationInterval::Inclusive(range) => {
                        let from = range.lower_bound;
                        let to = range.upper_bound;

                        (from..=to)
                            .map(|i: u32| {
                                let (pool_id, stake_pool_data) =
                                    &stake_pools[usize::try_from(i).unwrap()];
                                (
                                    Pool::new_with_data(
                                        certificate::PoolId::clone(pool_id),
                                        Arc::clone(stake_pool_data),
                                    ),
                                    i,
                                )
                            })
                            .collect::<Vec<(Pool, u32)>>()
                    }
                };

                connection.edges.extend(
                    edges
                        .iter()
                        .map(|(pool, cursor)| Edge::new(IndexCursor::from(*cursor), pool.clone())),
                );

                Ok::<_, async_graphql::Error>(connection)
            },
        )
        .await
    }

    /// Get a paginated view of all the blocks in this epoch
    pub async fn blocks_by_epoch(
        &self,
        context: &Context<'_>,
        epoch: EpochNumber,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<String>,
        after: Option<String>,
    ) -> FieldResult<
        Option<Connection<IndexCursor, Block, ConnectionFields<BlockCount>, EmptyFields>>,
    > {
        let epoch_data = match extract_context(context).db.get_epoch(epoch.0).await {
            Some(epoch_data) => epoch_data,
            None => return Ok(None),
        };

        Some(
            query(
                after,
                before,
                first,
                last,
                |after, before, first, last| async move {
                    let epoch_lower_bound = self
                        .state
                        .state()
                        .blocks
                        .lookup(&epoch_data.first_block)
                        .map(|block| u32::from(block.chain_length))
                        .expect("Epoch lower bound");

                    let epoch_upper_bound = self
                        .state
                        .state()
                        .blocks
                        .lookup(&epoch_data.last_block)
                        .map(|block| u32::from(block.chain_length))
                        .expect("Epoch upper bound");

                    let boundaries = PaginationInterval::Inclusive(InclusivePaginationInterval {
                        lower_bound: 0,
                        upper_bound: epoch_upper_bound.checked_sub(epoch_lower_bound).expect(
                            "pagination upper_bound to be greater or equal than lower_bound",
                        ),
                    });

                    let pagination_arguments = ValidatedPaginationArguments {
                        first,
                        last,
                        before: before.map(u32::try_from).transpose()?,
                        after: after.map(u32::try_from).transpose()?,
                    };

                    let (range, page_meta) = compute_interval(boundaries, pagination_arguments)?;
                    let mut connection = Connection::with_additional_fields(
                        page_meta.has_previous_page,
                        page_meta.has_next_page,
                        ConnectionFields {
                            total_count: page_meta.total_count,
                        },
                    );

                    let edges = match range {
                        PaginationInterval::Empty => {
                            unreachable!("No blocks found (not even genesis)")
                        }
                        PaginationInterval::Inclusive(range) => self
                            .state
                            .state()
                            .get_block_hash_range(
                                (range.lower_bound + epoch_lower_bound).into(),
                                (range.upper_bound + epoch_lower_bound + 1u32).into(),
                            )
                            .iter()
                            .map(|(hash, index)| (*hash, u32::from(*index) - epoch_lower_bound))
                            .collect::<Vec<_>>(),
                    };

                    connection.edges.extend(edges.iter().map(|(id, cursor)| {
                        Edge::new(IndexCursor::from(*cursor), Block::from_valid_hash(*id))
                    }));

                    Ok::<_, async_graphql::Error>(connection)
                },
            )
            .await,
        )
        .transpose()
    }
}

pub struct Block {
    hash: HeaderHash,
    contents: tokio::sync::Mutex<Option<Arc<ExplorerBlock>>>,
}

impl Block {
    async fn from_string_hash(hash: String, db: &ExplorerDb) -> FieldResult<Block> {
        let hash = HeaderHash::from_str(&hash)?;
        let block = Block {
            hash,
            contents: Default::default(),
        };

        block.fetch_explorer_block(db).await.map(|_| block)
    }

    fn from_valid_hash(hash: HeaderHash) -> Block {
        Block {
            hash,
            contents: Default::default(),
        }
    }

    fn from_contents(block: Arc<ExplorerBlock>) -> Block {
        Block {
            hash: block.id(),
            contents: tokio::sync::Mutex::new(Some(block)),
        }
    }

    async fn fetch_explorer_block(&self, db: &ExplorerDb) -> FieldResult<Arc<ExplorerBlock>> {
        let mut contents = self.contents.lock().await;
        if let Some(block) = &*contents {
            Ok(Arc::clone(block))
        } else {
            let block = db.get_block(&self.hash).await.ok_or_else(|| {
                ApiError::InternalError("Couldn't find block in the explorer".to_owned())
            })?;

            *contents = Some(Arc::clone(&block));
            Ok(block)
        }
    }

    async fn get_branches(&self, db: &ExplorerDb) -> FieldResult<Vec<Branch>> {
        let (block, mut branches) =
            db.get_block_with_branches(&self.hash)
                .await
                .ok_or_else(|| {
                    ApiError::InternalError("Couldn't find block in the explorer".to_owned())
                })?;

        let mut contents = self.contents.lock().await;
        contents.get_or_insert(block);

        Ok(branches
            .drain(..)
            .map(|(hash, state)| Branch::from_id_and_state(hash, state))
            .collect())
    }
}

/// A Block
#[Object]
impl Block {
    /// The Block unique identifier
    pub async fn id(&self) -> String {
        format!("{}", self.hash)
    }

    /// Date the Block was included in the blockchain
    pub async fn date(&self, context: &Context<'_>) -> FieldResult<BlockDate> {
        self.fetch_explorer_block(&extract_context(context).db)
            .await
            .map(|b| b.date().into())
    }

    /// The transactions contained in the block
    pub async fn transactions(
        &self,
        context: &Context<'_>,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<String>,
        after: Option<String>,
    ) -> FieldResult<
        Connection<IndexCursor, Transaction, ConnectionFields<TransactionCount>, EmptyFields>,
    > {
        let explorer_block = self
            .fetch_explorer_block(&extract_context(context).db)
            .await?;

        let mut transactions: Vec<&ExplorerTransaction> =
            explorer_block.transactions.values().collect();

        // TODO: This may be expensive at some point, but I can't rely in
        // the HashMap's order (also, I'm assuming the order in the block matters)
        transactions
            .as_mut_slice()
            .sort_unstable_by_key(|tx| tx.offset_in_block);

        query(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let pagination_arguments = ValidatedPaginationArguments {
                    first,
                    last,
                    before: before.map(u32::try_from).transpose()?,
                    after: after.map(u32::try_from).transpose()?,
                };

                let boundaries = if !transactions.is_empty() {
                    PaginationInterval::Inclusive(InclusivePaginationInterval {
                        lower_bound: 0u32,
                        upper_bound: transactions
                            .len()
                            .checked_sub(1)
                            .unwrap()
                            .try_into()
                            .expect("tried to paginate more than 2^32 elements"),
                    })
                } else {
                    PaginationInterval::Empty
                };

                let (range, page_meta) = compute_interval(boundaries, pagination_arguments)?;
                let mut connection = Connection::with_additional_fields(
                    page_meta.has_previous_page,
                    page_meta.has_next_page,
                    ConnectionFields {
                        total_count: page_meta.total_count,
                    },
                );

                let edges = match range {
                    PaginationInterval::Empty => vec![],
                    PaginationInterval::Inclusive(range) => {
                        let from = usize::try_from(range.lower_bound).unwrap();
                        let to = usize::try_from(range.upper_bound).unwrap();

                        (from..=to)
                            .map(|i| {
                                (
                                    Transaction::from_contents(transactions[i].clone()),
                                    i.try_into().unwrap(),
                                )
                            })
                            .collect::<Vec<(_, u32)>>()
                    }
                };

                connection.edges.extend(
                    edges
                        .iter()
                        .map(|(tx, cursor)| Edge::new(IndexCursor::from(*cursor), tx.clone())),
                );

                Ok::<_, async_graphql::Error>(connection)
            },
        )
        .await
    }

    pub async fn chain_length(&self, context: &Context<'_>) -> FieldResult<ChainLength> {
        self.fetch_explorer_block(&extract_context(context).db)
            .await
            .map(|block| ChainLength(block.chain_length()))
    }

    pub async fn leader(&self, context: &Context<'_>) -> FieldResult<Option<Leader>> {
        self.fetch_explorer_block(&extract_context(context).db)
            .await
            .map(|block| match block.producer() {
                BlockProducer::StakePool(pool) => {
                    Some(Leader::StakePool(Pool::from_valid_id(pool.clone())))
                }
                BlockProducer::BftLeader(id) => {
                    Some(Leader::BftLeader(BftLeader { id: id.clone() }))
                }
                BlockProducer::None => None,
            })
    }

    pub async fn previous_block(&self, context: &Context<'_>) -> FieldResult<Block> {
        self.fetch_explorer_block(&extract_context(context).db)
            .await
            .map(|b| Block::from_valid_hash(b.parent_hash))
    }

    pub async fn total_input(&self, context: &Context<'_>) -> FieldResult<Value> {
        self.fetch_explorer_block(&extract_context(context).db)
            .await
            .map(|block| Value(block.total_input))
    }

    pub async fn total_output(&self, context: &Context<'_>) -> FieldResult<Value> {
        self.fetch_explorer_block(&extract_context(context).db)
            .await
            .map(|block| Value(block.total_output))
    }

    pub async fn is_confirmed(&self, context: &Context<'_>) -> bool {
        extract_context(context)
            .db
            .is_block_confirmed(&self.hash)
            .await
    }

    pub async fn branches(&self, context: &Context<'_>) -> FieldResult<Vec<Branch>> {
        let branches = self.get_branches(&extract_context(context).db).await?;

        Ok(branches)
    }
}

#[derive(Clone)]
pub struct BftLeader {
    id: BftLeaderId,
}

impl From<BftLeaderId> for BftLeader {
    fn from(id: BftLeaderId) -> Self {
        Self { id }
    }
}

#[Object]
impl BftLeader {
    async fn id(&self) -> PublicKey {
        self.id.as_public_key().into()
    }
}

#[derive(Union)]
pub enum Leader {
    StakePool(Pool),
    BftLeader(BftLeader),
}

impl From<Arc<ExplorerBlock>> for Block {
    fn from(block: Arc<ExplorerBlock>) -> Block {
        Block::from_valid_hash(block.id())
    }
}

/// Block's date, composed of an Epoch and a Slot
#[derive(Clone, SimpleObject)]
pub struct BlockDate {
    epoch: Epoch,
    slot: Slot,
}

impl From<InternalBlockDate> for BlockDate {
    fn from(date: InternalBlockDate) -> BlockDate {
        BlockDate {
            epoch: Epoch { id: date.epoch },
            slot: Slot(date.slot_id),
        }
    }
}

#[derive(Clone)]
pub struct Transaction {
    id: FragmentId,
    block_hashes: Vec<HeaderHash>,
    contents: Option<ExplorerTransaction>,
}

impl Transaction {
    async fn from_id(id: FragmentId, context: &Context<'_>) -> FieldResult<Transaction> {
        let block_hashes = extract_context(context)
            .db
            .find_blocks_by_transaction(&id)
            .await;

        if block_hashes.is_empty() {
            Err(ApiError::NotFound(format!("transaction not found: {}", &id,)).into())
        } else {
            Ok(Transaction {
                id,
                block_hashes,
                contents: None,
            })
        }
    }

    fn from_valid_id(id: FragmentId) -> Transaction {
        Transaction {
            id,
            block_hashes: Default::default(),
            contents: None,
        }
    }

    fn from_contents(contents: ExplorerTransaction) -> Transaction {
        Transaction {
            id: contents.id,
            block_hashes: Default::default(),
            contents: Some(contents),
        }
    }

    async fn get_blocks(&self, context: &Context<'_>) -> FieldResult<Vec<Arc<ExplorerBlock>>> {
        let block_ids = if self.block_hashes.is_empty() {
            extract_context(context)
                .db
                .find_blocks_by_transaction(&self.id)
                .await
        } else {
            self.block_hashes.clone()
        };

        if block_ids.is_empty() {
            return Err(FieldError::from(ApiError::InternalError(
                "Transaction is not present in any block".to_owned(),
            )));
        }

        let mut result = Vec::new();

        for block_id in block_ids {
            let block = extract_context(context)
                .db
                .get_block(&block_id)
                .await
                .ok_or_else(|| {
                    FieldError::from(ApiError::InternalError(
                        "transaction is in explorer but couldn't find its block".to_owned(),
                    ))
                })?;

            result.push(block);
        }

        Ok(result)
    }

    async fn get_contents(&self, context: &Context<'_>) -> FieldResult<ExplorerTransaction> {
        if let Some(c) = &self.contents {
            Ok(c.clone())
        } else {
            //TODO: maybe store transactions outside blocks? as Arc, as doing it this way is pretty wasty

            let block = extract_context(context)
                .db
                .get_block(&self.block_hashes[0])
                .await
                .ok_or_else(|| {
                    FieldError::from(ApiError::InternalError(
                        "failed to fetch block containing the transaction".to_owned(),
                    ))
                })?;

            Ok(block
                .transactions
                .get(&self.id)
                .ok_or_else(|| {
                    ApiError::InternalError(
                        "transaction was not found in respective block".to_owned(),
                    )
                })?
                .clone())
        }
    }
}

/// A transaction in the blockchain
#[Object]
impl Transaction {
    /// The hash that identifies the transaction
    pub async fn id(&self) -> String {
        format!("{}", self.id)
    }

    /// All the blocks this transaction is included in
    pub async fn blocks(&self, context: &Context<'_>) -> FieldResult<Vec<Block>> {
        let blocks = self.get_blocks(context).await?;

        Ok(blocks.iter().map(|b| Block::from(Arc::clone(b))).collect())
    }

    /// Initial bootstrap config params (initial fragments), only present in Block0
    pub async fn initial_configuration_params(
        &self,
        context: &Context<'_>,
    ) -> FieldResult<Option<config_param::ConfigParams>> {
        let transaction = self.get_contents(context).await?;
        match transaction.config_params {
            Some(params) => Ok(Some(config_param::ConfigParams::from(&params))),
            None => Ok(None),
        }
    }

    pub async fn inputs(&self, context: &Context<'_>) -> FieldResult<Vec<TransactionInput>> {
        let transaction = self.get_contents(context).await?;
        Ok(transaction
            .inputs()
            .iter()
            .map(|input| TransactionInput {
                address: Address::from(&input.address),
                amount: Value(input.value),
            })
            .collect())
    }

    pub async fn outputs(&self, context: &Context<'_>) -> FieldResult<Vec<TransactionOutput>> {
        let transaction = self.get_contents(context).await?;
        Ok(transaction
            .outputs()
            .iter()
            .map(|input| TransactionOutput {
                address: Address::from(&input.address),
                amount: Value(input.value),
            })
            .collect())
    }

    pub async fn certificate(
        &self,
        context: &Context<'_>,
    ) -> FieldResult<Option<certificates::Certificate>> {
        self.get_contents(context)
            .await
            .map(|transaction| transaction.certificate.map(Certificate::from))
    }
}

#[derive(SimpleObject)]
pub struct TransactionInput {
    amount: Value,
    address: Address,
}

#[derive(SimpleObject)]
pub struct TransactionOutput {
    amount: Value,
    address: Address,
}

#[derive(Clone)]
pub struct Address {
    id: ExplorerAddress,
}

impl Address {
    fn from_bech32(bech32: &str) -> FieldResult<Address> {
        let addr = chain_addr::AddressReadable::from_string_anyprefix(bech32)
            .map(|adr| ExplorerAddress::New(adr.to_address()))
            .or_else(|_| OldAddress::from_str(bech32).map(ExplorerAddress::Old))
            .map_err(|_| ApiError::InvalidAddress(bech32.to_string()))?;

        Ok(Address { id: addr })
    }
}

impl From<&ExplorerAddress> for Address {
    fn from(addr: &ExplorerAddress) -> Address {
        Address { id: addr.clone() }
    }
}

#[Object]
impl Address {
    /// The base32 representation of an address
    async fn id(&self, context: &Context<'_>) -> String {
        match &self.id {
            ExplorerAddress::New(addr) => chain_addr::AddressReadable::from_address(
                &extract_context(context).settings.address_bech32_prefix,
                addr,
            )
            .to_string(),
            ExplorerAddress::Old(addr) => format!("{}", addr),
        }
    }

    async fn delegation(&self, _context: &Context<'_>) -> FieldResult<Pool> {
        Err(ApiError::Unimplemented.into())
    }
}

pub struct TaxType(chain_impl_mockchain::rewards::TaxType);

#[Object]
impl TaxType {
    /// what get subtracted as fixed value
    pub async fn fixed(&self) -> Value {
        Value(self.0.fixed)
    }
    /// Ratio of tax after fixed amout subtracted
    pub async fn ratio(&self) -> Ratio {
        Ratio(self.0.ratio)
    }

    /// Max limit of tax
    pub async fn max_limit(&self) -> Option<NonZero> {
        self.0.max_limit.map(NonZero)
    }
}

pub struct Ratio(chain_impl_mockchain::rewards::Ratio);

#[Object]
impl Ratio {
    pub async fn numerator(&self) -> Value {
        Value::from(self.0.numerator)
    }

    pub async fn denominator(&self) -> NonZero {
        NonZero(self.0.denominator)
    }
}

pub struct Proposal(certificate::Proposal);

#[Object]
impl Proposal {
    pub async fn external_id(&self) -> ExternalProposalId {
        ExternalProposalId(self.0.external_id().to_string())
    }

    /// get the vote options range
    ///
    /// this is the available range of choices to make for the given
    /// proposal. all casted votes for this proposals ought to be in
    /// within the given range
    pub async fn options(&self) -> VoteOptionRange {
        self.0.options().clone().into()
    }
}

#[derive(Clone)]
pub struct Pool {
    id: certificate::PoolId,
    data: Option<Arc<StakePoolData>>,
    blocks: Option<Arc<PersistentSequence<HeaderHash>>>,
}

impl Pool {
    async fn from_string_id(id: &str, db: &ExplorerDb) -> FieldResult<Pool> {
        let id = certificate::PoolId::from_str(id)?;
        let blocks = db
            .get_stake_pool_blocks(&id)
            .await
            .ok_or_else(|| ApiError::NotFound("Stake pool not found".to_owned()))?;

        let data = db
            .get_stake_pool_data(&id)
            .await
            .ok_or_else(|| ApiError::NotFound("Stake pool not found".to_owned()))?;

        Ok(Pool {
            id,
            data: Some(data),
            blocks: Some(blocks),
        })
    }

    fn from_valid_id(id: certificate::PoolId) -> Pool {
        Pool {
            id,
            blocks: None,
            data: None,
        }
    }

    fn new_with_data(id: certificate::PoolId, data: Arc<StakePoolData>) -> Self {
        Pool {
            id,
            blocks: None,
            data: Some(data),
        }
    }
}

#[Object]
impl Pool {
    pub async fn id(&self) -> PoolId {
        PoolId(self.id.clone())
    }

    pub async fn blocks(
        &self,
        context: &Context<'_>,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<String>,
        after: Option<String>,
    ) -> FieldResult<Connection<IndexCursor, Block, ConnectionFields<BlockCount>>> {
        let blocks = match &self.blocks {
            Some(b) => b.clone(),
            None => extract_context(context)
                .db
                .get_stake_pool_blocks(&self.id)
                .await
                .ok_or_else(|| {
                    ApiError::InternalError("Stake pool in block is not indexed".to_owned())
                })?,
        };

        query(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let bounds = if blocks.len() > 0 {
                    PaginationInterval::Inclusive(InclusivePaginationInterval {
                        lower_bound: 0u32,
                        upper_bound: blocks
                            .len()
                            .checked_sub(1)
                            .unwrap()
                            .try_into()
                            .expect("Tried to paginate more than 2^32 blocks"),
                    })
                } else {
                    PaginationInterval::Empty
                };

                let pagination_arguments = ValidatedPaginationArguments {
                    first,
                    last,
                    before: before.map(u32::try_from).transpose()?,
                    after: after.map(u32::try_from).transpose()?,
                };

                let (range, page_meta) = compute_interval(bounds, pagination_arguments)?;

                let edges = match range {
                    PaginationInterval::Empty => vec![],
                    PaginationInterval::Inclusive(range) => (range.lower_bound..=range.upper_bound)
                        .filter_map(|i| blocks.get(i).map(|h| (*h.as_ref(), i)))
                        .collect(),
                };

                let mut connection = Connection::with_additional_fields(
                    page_meta.has_previous_page,
                    page_meta.has_next_page,
                    ConnectionFields {
                        total_count: page_meta.total_count,
                    },
                );

                connection.edges.extend(
                    edges
                        .iter()
                        .map(|(h, i)| Edge::new(IndexCursor::from(*i), Block::from_valid_hash(*h))),
                );

                Ok::<_, async_graphql::Error>(connection)
            },
        )
        .await
    }

    pub async fn registration(&self, context: &Context<'_>) -> FieldResult<PoolRegistration> {
        match &self.data {
            Some(data) => Ok(data.registration.clone().into()),
            None => extract_context(context)
                .db
                .get_stake_pool_data(&self.id)
                .await
                .map(|data| PoolRegistration::from(data.registration.clone()))
                .ok_or_else(|| ApiError::NotFound("Stake pool not found".to_owned()).into()),
        }
    }

    pub async fn retirement(&self, context: &Context<'_>) -> FieldResult<Option<PoolRetirement>> {
        match &self.data {
            Some(data) => Ok(data.retirement.clone().map(PoolRetirement::from)),
            None => extract_context(context)
                .db
                .get_stake_pool_data(&self.id)
                .await
                .ok_or_else(|| ApiError::NotFound("Stake pool not found".to_owned()).into())
                .map(|data| {
                    data.retirement
                        .as_ref()
                        .map(|r| PoolRetirement::from(r.clone()))
                }),
        }
    }
}

pub struct Settings {}

#[Object]
impl Settings {
    pub async fn fees(&self, context: &Context<'_>) -> LinearFee {
        From::from(&extract_context(context).db.blockchain_config.fees)
    }

    pub async fn epoch_stability_depth(&self, context: &Context<'_>) -> EpochStabilityDepth {
        From::from(
            &extract_context(context)
                .db
                .blockchain_config
                .epoch_stability_depth,
        )
    }
}

#[derive(SimpleObject)]
pub struct Treasury {
    rewards: Value,
    treasury: Value,
    treasury_tax: TaxType,
}

#[derive(SimpleObject)]
pub struct FeeSettings {
    constant: Value,
    coefficient: Value,
    certificate: Value,
    certificate_pool_registration: Value,
    certificate_stake_delegation: Value,
    certificate_owner_stake_delegation: Value,
    certificate_vote_plan: Value,
    certificate_vote_cast: Value,
}

#[derive(Clone)]
pub struct Epoch {
    id: InternalEpoch,
}

impl Epoch {
    fn from_epoch_number(id: InternalEpoch) -> Epoch {
        Epoch { id }
    }

    async fn get_epoch_data(&self, db: &ExplorerDb) -> Option<EpochData> {
        db.get_epoch(self.id).await
    }
}

#[Object]
impl Epoch {
    pub async fn id(&self) -> EpochNumber {
        EpochNumber(self.id)
    }

    /// Not yet implemented
    pub async fn stake_distribution(&self) -> FieldResult<StakeDistribution> {
        Err(ApiError::Unimplemented.into())
    }

    pub async fn first_block(&self, context: &Context<'_>) -> Option<Block> {
        self.get_epoch_data(&extract_context(context).db)
            .await
            .map(|data| Block::from_valid_hash(data.first_block))
    }

    pub async fn last_block(&self, context: &Context<'_>) -> Option<Block> {
        self.get_epoch_data(&extract_context(context).db)
            .await
            .map(|data| Block::from_valid_hash(data.last_block))
    }

    pub async fn total_blocks(&self, context: &Context<'_>) -> BlockCount {
        self.get_epoch_data(&extract_context(context).db)
            .await
            .map_or(0u32.into(), |data| data.total_blocks.into())
    }
}

#[derive(SimpleObject)]
pub struct StakeDistribution {
    pools: Vec<PoolStakeDistribution>,
}

#[derive(SimpleObject)]
pub struct PoolStakeDistribution {
    pool: Pool,
    delegated_stake: Value,
}

#[derive(Clone)]
pub struct VotePayloadPublicStatus {
    choice: i32,
}

#[derive(Clone)]
pub struct VotePayloadPrivateStatus {
    proof: ProofOfCorrectVote,
    encrypted_vote: EncryptedVote,
}

#[Object]
impl VotePayloadPublicStatus {
    pub async fn choice(&self, _context: &Context<'_>) -> i32 {
        self.choice
    }
}

#[Object]
impl VotePayloadPrivateStatus {
    pub async fn proof(&self, _context: &Context<'_>) -> String {
        let bytes_proof = self.proof.serialize();
        base64::encode_config(bytes_proof, base64::URL_SAFE)
    }

    pub async fn encrypted_vote(&self, _context: &Context<'_>) -> String {
        let encrypted_bote_bytes = self.encrypted_vote.serialize();
        base64::encode_config(encrypted_bote_bytes, base64::URL_SAFE)
    }
}

#[derive(Clone, Union)]
pub enum VotePayloadStatus {
    Public(VotePayloadPublicStatus),
    Private(VotePayloadPrivateStatus),
}

// TODO do proper vote tally
#[derive(Clone, SimpleObject)]
pub struct TallyPublicStatus {
    results: Vec<Weight>,
    options: VoteOptionRange,
}

#[derive(Clone, SimpleObject)]
pub struct TallyPrivateStatus {
    results: Option<Vec<Weight>>,
    options: VoteOptionRange,
}

#[derive(Clone, Union)]
pub enum TallyStatus {
    Public(TallyPublicStatus),
    Private(TallyPrivateStatus),
}

#[derive(Clone, SimpleObject)]
pub struct VotePlanStatus {
    id: VotePlanId,
    vote_start: BlockDate,
    vote_end: BlockDate,
    committee_end: BlockDate,
    payload_type: PayloadType,
    proposals: Vec<VoteProposalStatus>,
}

impl VotePlanStatus {
    pub async fn vote_plan_from_id(
        vote_plan_id: VotePlanId,
        context: &Context<'_>,
    ) -> FieldResult<Self> {
        let vote_plan_id = chain_impl_mockchain::certificate::VotePlanId::from_str(&vote_plan_id.0)
            .map_err(|err| -> FieldError { ApiError::InvalidAddress(err.to_string()).into() })?;
        if let Some(vote_plan) = extract_context(context)
            .db
            .get_vote_plan_by_id(&vote_plan_id)
            .await
        {
            return Ok(Self::vote_plan_from_data(vote_plan));
        }

        Err(ApiError::NotFound(format!("Vote plan with id {} not found", vote_plan_id)).into())
    }

    pub fn vote_plan_from_data(vote_plan: Arc<ExplorerVotePlan>) -> Self {
        let ExplorerVotePlan {
            id,
            vote_start,
            vote_end,
            committee_end,
            payload_type,
            proposals,
        } = (*vote_plan).clone();

        VotePlanStatus {
            id: VotePlanId::from(id),
            vote_start: BlockDate::from(vote_start),
            vote_end: BlockDate::from(vote_end),
            committee_end: BlockDate::from(committee_end),
            payload_type: PayloadType::from(payload_type),
            proposals: proposals
                .into_iter()
                .map(|proposal| VoteProposalStatus {
                    proposal_id: ExternalProposalId::from(proposal.proposal_id),
                    options: VoteOptionRange::from(proposal.options),
                    tally: proposal.tally.map(|tally| match tally {
                        ExplorerVoteTally::Public { results, options } => {
                            TallyStatus::Public(TallyPublicStatus {
                                results: results.iter().map(Into::into).collect(),
                                options: options.into(),
                            })
                        }
                        ExplorerVoteTally::Private { results, options } => {
                            TallyStatus::Private(TallyPrivateStatus {
                                results: results.map(|res| res.iter().map(Into::into).collect()),
                                options: options.into(),
                            })
                        }
                    }),
                    votes: proposal
                        .votes
                        .iter()
                        .map(|(key, vote)| match vote.as_ref() {
                            ExplorerVote::Public(choice) => VoteStatus {
                                address: key.into(),
                                payload: VotePayloadStatus::Public(VotePayloadPublicStatus {
                                    choice: choice.as_byte().into(),
                                }),
                            },
                            ExplorerVote::Private {
                                proof,
                                encrypted_vote,
                            } => VoteStatus {
                                address: key.into(),
                                payload: VotePayloadStatus::Private(VotePayloadPrivateStatus {
                                    proof: proof.clone(),
                                    encrypted_vote: encrypted_vote.clone(),
                                }),
                            },
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

#[derive(Clone, SimpleObject)]
pub struct VoteStatus {
    address: Address,
    payload: VotePayloadStatus,
}

#[derive(Clone)]
pub struct VoteProposalStatus {
    proposal_id: ExternalProposalId,
    options: VoteOptionRange,
    tally: Option<TallyStatus>,
    votes: Vec<VoteStatus>,
}

#[Object]
impl VoteProposalStatus {
    pub async fn proposal_id(&self) -> &ExternalProposalId {
        &self.proposal_id
    }

    pub async fn options(&self) -> &VoteOptionRange {
        &self.options
    }

    pub async fn tally(&self) -> Option<&TallyStatus> {
        self.tally.as_ref()
    }

    pub async fn votes(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<String>,
        after: Option<String>,
    ) -> FieldResult<Connection<IndexCursor, VoteStatus, ConnectionFields<u64>, EmptyFields>> {
        query(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let boundaries = if !self.votes.is_empty() {
                    PaginationInterval::Inclusive(InclusivePaginationInterval {
                        lower_bound: 0u32,
                        upper_bound: self
                            .votes
                            .len()
                            .checked_sub(1)
                            .unwrap()
                            .try_into()
                            .expect("tried to paginate more than 2^32 elements"),
                    })
                } else {
                    PaginationInterval::Empty
                };

                let pagination_arguments = ValidatedPaginationArguments {
                    first,
                    last,
                    before: before.map(u32::try_from).transpose()?,
                    after: after.map(u32::try_from).transpose()?,
                };

                let (range, page_meta) = compute_interval(boundaries, pagination_arguments)?;
                let mut connection = Connection::with_additional_fields(
                    page_meta.has_previous_page,
                    page_meta.has_next_page,
                    ConnectionFields {
                        total_count: page_meta.total_count,
                    },
                );

                let edges = match range {
                    PaginationInterval::Empty => vec![],
                    PaginationInterval::Inclusive(range) => {
                        let from = range.lower_bound;
                        let to = range.upper_bound;

                        (from..=to)
                            .map(|i: u32| (self.votes[i as usize].clone(), i))
                            .collect::<Vec<(VoteStatus, u32)>>()
                    }
                };

                connection.edges.extend(
                    edges
                        .iter()
                        .map(|(vs, cursor)| Edge::new(IndexCursor::from(*cursor), vs.clone())),
                );

                Ok::<_, async_graphql::Error>(connection)
            },
        )
        .await
    }
}

pub struct Query;

#[Object]
impl Query {
    async fn block(&self, context: &Context<'_>, id: String) -> FieldResult<Block> {
        Block::from_string_hash(id, &extract_context(context).db).await
    }

    async fn blocks_by_chain_length(
        &self,
        context: &Context<'_>,
        length: ChainLength,
    ) -> FieldResult<Vec<Block>> {
        let blocks = extract_context(context)
            .db
            .find_blocks_by_chain_length(length.0)
            .await
            .iter()
            .cloned()
            .map(Block::from_valid_hash)
            .collect();

        Ok(blocks)
    }

    async fn transaction(&self, context: &Context<'_>, id: String) -> FieldResult<Transaction> {
        let id = FragmentId::from_str(&id)?;

        Transaction::from_id(id, context).await
    }

    /// get all current tips, sorted (descending) by their length
    pub async fn branches(&self, context: &Context<'_>) -> Vec<Branch> {
        extract_context(context)
            .db
            .get_branches()
            .await
            .iter()
            .cloned()
            .map(|(id, state_ref)| Branch::from_id_and_state(id, state_ref))
            .collect()
    }

    /// get the block that the ledger currently considers as the main branch's
    /// tip
    async fn tip(&self, context: &Context<'_>) -> Branch {
        let (hash, state_ref) = extract_context(context).db.get_tip().await;
        Branch::from_id_and_state(hash, state_ref)
    }

    pub async fn branch(&self, context: &Context<'_>, id: String) -> FieldResult<Branch> {
        let id = HeaderHash::from_str(&id)?;
        Branch::try_from_id(id, extract_context(context)).await
    }

    pub async fn epoch(&self, _context: &Context<'_>, id: EpochNumber) -> Epoch {
        Epoch::from_epoch_number(id.0)
    }

    pub async fn address(&self, _context: &Context<'_>, bech32: String) -> FieldResult<Address> {
        Address::from_bech32(&bech32)
    }

    pub async fn stake_pool(&self, context: &Context<'_>, id: PoolId) -> FieldResult<Pool> {
        Pool::from_string_id(&id.0.to_string(), &extract_context(context).db).await
    }

    pub async fn settings(&self, _context: &Context<'_>) -> FieldResult<Settings> {
        Ok(Settings {})
    }

    pub async fn vote_plan(
        &self,
        context: &Context<'_>,
        id: String,
    ) -> FieldResult<VotePlanStatus> {
        VotePlanStatus::vote_plan_from_id(VotePlanId(id), context).await
    }
}

pub struct Subscription;

#[Subscription]
impl Subscription {
    async fn tip(&self, context: &Context<'_>) -> impl futures::Stream<Item = Branch> {
        use futures::StreamExt;
        extract_context(context)
            .db
            .tip_subscription()
            // missing a tip update doesn't seem that important, so I think it's
            // fine to ignore the error
            .filter_map(|tip| async move {
                tip.ok()
                    .map(|(hash, state)| Branch::from_id_and_state(hash, state))
            })
    }
}

pub type Schema = async_graphql::Schema<Query, EmptyMutation, Subscription>;

pub struct EContext {
    pub db: ExplorerDb,
    pub settings: ChainSettings,
}

fn extract_context<'a>(context: &Context<'a>) -> &'a EContext {
    context.data_unchecked::<EContext>()
}

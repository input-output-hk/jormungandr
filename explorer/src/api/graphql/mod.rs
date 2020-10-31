mod certificates;
mod connections;
mod error;
mod scalars;

use self::connections::{
    BlockConnection, InclusivePaginationInterval, PaginationArguments, PaginationInterval,
    PoolConnection, TransactionConnection, TransactionNodeFetchInfo,
};
use self::error::Error;
use crate::db::indexing::{
    BlockProducer, EpochData, ExplorerAddress, ExplorerBlock, ExplorerTransaction, StakePoolData,
};
use crate::db::persistent_sequence::PersistentSequence;
use cardano_legacy_address::Addr as OldAddress;
use certificates::*;
use chain_impl_mockchain::certificate;

use chain_impl_mockchain::fragment::FragmentId;
use chain_impl_mockchain::header;
use chain_impl_mockchain::header::HeaderId as HeaderHash;
use chain_impl_mockchain::key::BftLeaderId;
use futures::executor::block_on;
pub use juniper::http::GraphQLRequest;
use juniper::{graphql_union, EmptyMutation, FieldResult, RootNode};
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;

use self::scalars::{
    BlockCount, ChainLength, EpochNumber, ExternalProposalId, IndexCursor, NonZero, PoolId,
    PublicKey, Slot, Value, VoteOptionRange,
};

use crate::db::DB;

#[derive(Clone)]
pub struct GraphQLSettings {
    /// This is the prefix that's used for the Address bech32 string representation in the
    /// responses (in the queries any prefix can be used). base32 serialization could
    /// also be used, but the `Address` struct doesn't have a deserialization method right
    /// now
    pub address_bech32_prefix: String,
}

#[derive(Clone)]
pub struct Block {
    hash: HeaderHash,
}

impl Block {
    fn from_string_hash(hash: String, db: &DB) -> FieldResult<Block> {
        let hash = HeaderHash::from_str(&hash)?;
        let block = Block { hash };

        block.get_explorer_block(db).map(|_| block)
    }

    fn from_valid_hash(hash: HeaderHash) -> Block {
        Block { hash }
    }

    fn get_explorer_block(&self, db: &DB) -> FieldResult<ExplorerBlock> {
        block_on(db.get_block(&self.hash)).ok_or_else(|| {
            Error::InternalError("Couldn't find block's contents in explorer".to_owned()).into()
        })
    }
}

/// A Block
#[juniper::object(
    Context = Context
)]
impl Block {
    /// The Block unique identifier
    pub fn id(&self) -> String {
        format!("{}", self.hash)
    }

    /// Date the Block was included in the blockchain
    pub fn date(&self, context: &Context) -> FieldResult<BlockDate> {
        self.get_explorer_block(&context.db)
            .map(|b| b.date().into())
    }

    /// The transactions contained in the block
    pub fn transactions(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<IndexCursor>,
        after: Option<IndexCursor>,
        context: &Context,
    ) -> FieldResult<TransactionConnection> {
        let explorer_block = self.get_explorer_block(&context.db)?;
        let mut transactions: Vec<&ExplorerTransaction> =
            explorer_block.transactions.values().collect();

        // TODO: This may be expensive at some point, but I can't rely in
        // the HashMap's order (also, I'm assuming the order in the block matters)
        transactions
            .as_mut_slice()
            .sort_unstable_by_key(|tx| tx.offset_in_block);

        let pagination_arguments = PaginationArguments {
            first,
            last,
            before: before.map(u32::try_from).transpose()?,
            after: after.map(u32::try_from).transpose()?,
        }
        .validate()?;

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

        TransactionConnection::new(
            boundaries,
            pagination_arguments,
            |range: PaginationInterval<u32>| match range {
                PaginationInterval::Empty => vec![],
                PaginationInterval::Inclusive(range) => {
                    let from = usize::try_from(range.lower_bound).unwrap();
                    let to = usize::try_from(range.upper_bound).unwrap();

                    (from..=to)
                        .map(|i| {
                            (
                                TransactionNodeFetchInfo::Contents(transactions[i].clone()),
                                i.try_into().unwrap(),
                            )
                        })
                        .collect::<Vec<(TransactionNodeFetchInfo, u32)>>()
                }
            },
        )
    }

    pub fn previous_block(&self, context: &Context) -> FieldResult<Block> {
        self.get_explorer_block(&context.db)
            .map(|b| Block::from_valid_hash(b.parent_hash))
    }

    pub fn chain_length(&self, context: &Context) -> FieldResult<ChainLength> {
        self.get_explorer_block(&context.db)
            .map(|block| block.chain_length().into())
    }

    pub fn leader(&self, context: &Context) -> FieldResult<Option<Leader>> {
        self.get_explorer_block(&context.db)
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

    pub fn total_input(&self, context: &Context) -> FieldResult<Value> {
        self.get_explorer_block(&context.db)
            .map(|block| Value(format!("{}", block.total_input)))
    }

    pub fn total_output(&self, context: &Context) -> FieldResult<Value> {
        self.get_explorer_block(&context.db)
            .map(|block| Value(format!("{}", block.total_output)))
    }
}

struct BftLeader {
    id: BftLeaderId,
}

#[juniper::object(
    Context = Context,
)]
impl BftLeader {
    fn id(&self) -> PublicKey {
        self.id.as_public_key().into()
    }
}

enum Leader {
    StakePool(Pool),
    BftLeader(BftLeader),
}

graphql_union!(Leader: Context |&self| {
    instance_resolvers: |_| {
        &Pool => match *self { Leader::StakePool(ref c) => Some(c), _ => None },
        &BftLeader => match *self { Leader::BftLeader(ref c) => Some(c), _ => None },
    }
});

impl From<&ExplorerBlock> for Block {
    fn from(block: &ExplorerBlock) -> Block {
        Block::from_valid_hash(block.id())
    }
}

#[derive(Clone)]
struct BlockDate {
    epoch: Epoch,
    slot: Slot,
}

/// Block's date, composed of an Epoch and a Slot
#[juniper::object(
    Context = Context
)]
impl BlockDate {
    pub fn epoch(&self) -> &Epoch {
        &self.epoch
    }

    pub fn slot(&self) -> &Slot {
        &self.slot
    }
}

impl From<header::BlockDate> for BlockDate {
    fn from(date: header::BlockDate) -> BlockDate {
        BlockDate {
            epoch: Epoch { id: date.epoch },
            slot: Slot(format!("{}", date.slot_id)),
        }
    }
}

#[derive(Clone)]
struct Transaction {
    id: FragmentId,
    block_hash: Option<HeaderHash>,
    contents: Option<ExplorerTransaction>,
}

impl Transaction {
    fn from_id(id: FragmentId, context: &Context) -> FieldResult<Transaction> {
        let block_hash = block_on(context.db.find_block_hash_by_transaction(&id))
            .ok_or_else(|| Error::NotFound(format!("transaction not found: {}", &id,)))?;

        Ok(Transaction {
            id,
            block_hash: Some(block_hash),
            contents: None,
        })
    }

    fn from_valid_id(id: FragmentId) -> Transaction {
        Transaction {
            id,
            block_hash: None,
            contents: None,
        }
    }

    fn from_contents(contents: ExplorerTransaction) -> Transaction {
        Transaction {
            id: contents.id,
            block_hash: None,
            contents: Some(contents),
        }
    }

    fn get_block(&self, context: &Context) -> FieldResult<ExplorerBlock> {
        let block_id =
            match self.block_hash {
                Some(block_id) => block_id,
                None => block_on(context.db.find_block_hash_by_transaction(&self.id)).ok_or_else(
                    || Error::InternalError("Transaction's block was not found".to_owned()),
                )?,
            };

        block_on(context.db.get_block(&block_id)).ok_or_else(|| {
            Error::InternalError(
                "transaction is in explorer but couldn't find its block".to_owned(),
            )
            .into()
        })
    }

    fn get_contents(&self, context: &Context) -> FieldResult<ExplorerTransaction> {
        if let Some(c) = &self.contents {
            Ok(c.clone())
        } else {
            let block = self.get_block(context)?;
            Ok(block
                .transactions
                .get(&self.id)
                .ok_or_else(|| {
                    Error::InternalError("transaction was not found in respective block".to_owned())
                })?
                .clone())
        }
    }
}

/// A transaction in the blockchain
#[juniper::object(
    Context = Context
)]
impl Transaction {
    /// The hash that identifies the transaction
    pub fn id(&self) -> String {
        format!("{}", self.id)
    }

    /// The block this transaction is in
    pub fn block(&self, context: &Context) -> FieldResult<Block> {
        let block = self.get_block(context)?;
        Ok(Block::from(&block))
    }

    pub fn inputs(&self, context: &Context) -> FieldResult<Vec<TransactionInput>> {
        let transaction = self.get_contents(context)?;
        Ok(transaction
            .inputs()
            .iter()
            .map(|input| TransactionInput {
                address: Address::from(&input.address),
                amount: Value::from(&input.value),
            })
            .collect())
    }

    pub fn outputs(&self, context: &Context) -> FieldResult<Vec<TransactionOutput>> {
        let transaction = self.get_contents(context)?;
        Ok(transaction
            .outputs()
            .iter()
            .map(|input| TransactionOutput {
                address: Address::from(&input.address),
                amount: Value::from(&input.value),
            })
            .collect())
    }

    pub fn certificate(&self, context: &Context) -> FieldResult<Option<certificates::Certificate>> {
        let transaction = self.get_contents(context)?;
        match transaction.certificate {
            Some(c) => Ok(Certificate::try_from(c).map(Some)?),
            None => Ok(None),
        }
    }
}

struct TransactionInput {
    amount: Value,
    address: Address,
}

#[juniper::object(
    Context = Context
)]
impl TransactionInput {
    fn amount(&self) -> &Value {
        &self.amount
    }

    fn address(&self) -> &Address {
        &self.address
    }
}

struct TransactionOutput {
    amount: Value,
    address: Address,
}

#[juniper::object(
    Context = Context
)]
impl TransactionOutput {
    fn amount(&self) -> &Value {
        &self.amount
    }

    fn address(&self) -> &Address {
        &self.address
    }
}

#[derive(Clone)]
struct Address {
    id: ExplorerAddress,
}

impl Address {
    fn from_bech32(bech32: &str) -> FieldResult<Address> {
        let addr = chain_addr::AddressReadable::from_string_anyprefix(bech32)
            .map(|adr| ExplorerAddress::New(adr.to_address()))
            .or_else(|_| OldAddress::from_str(bech32).map(ExplorerAddress::Old))
            .map_err(|_| Error::InvalidAddress(bech32.to_string()))?;

        Ok(Address { id: addr })
    }
}

impl From<&ExplorerAddress> for Address {
    fn from(addr: &ExplorerAddress) -> Address {
        Address { id: addr.clone() }
    }
}

#[juniper::object(
    Context = Context
)]
impl Address {
    /// The base32 representation of an address
    fn id(&self, context: &Context) -> String {
        match &self.id {
            ExplorerAddress::New(addr) => chain_addr::AddressReadable::from_address(
                &context.settings.address_bech32_prefix,
                addr,
            )
            .to_string(),
            ExplorerAddress::Old(addr) => format!("{}", addr),
        }
    }

    fn delegation() -> FieldResult<Pool> {
        Err(Error::Unimplemented("address delegation".to_owned()).into())
    }

    fn transactions(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<IndexCursor>,
        after: Option<IndexCursor>,
        context: &Context,
    ) -> FieldResult<TransactionConnection> {
        let transactions = block_on(context.db.get_transactions_by_address(&self.id))
            .unwrap_or_else(PersistentSequence::<FragmentId>::new);

        let boundaries = if transactions.len() > 0 {
            PaginationInterval::Inclusive(InclusivePaginationInterval {
                lower_bound: 0u64,
                upper_bound: transactions.len(),
            })
        } else {
            PaginationInterval::Empty
        };

        let pagination_arguments = PaginationArguments {
            first,
            last,
            before: before.map(u64::from),
            after: after.map(u64::from),
        }
        .validate()?;

        TransactionConnection::new(
            boundaries,
            pagination_arguments,
            |range: PaginationInterval<u64>| match range {
                PaginationInterval::Empty => vec![],
                PaginationInterval::Inclusive(range) => (range.lower_bound..=range.upper_bound)
                    .filter_map(|i| {
                        transactions
                            .get(i)
                            .map(|h| (TransactionNodeFetchInfo::Id(*h.as_ref()), i))
                    })
                    .collect(),
            },
        )
    }
}

struct TaxType(chain_impl_mockchain::rewards::TaxType);

#[juniper::object(
    Context = Context,
)]
impl TaxType {
    /// what get subtracted as fixed value
    pub fn fixed(&self) -> Value {
        Value(format!("{}", self.0.fixed))
    }
    /// Ratio of tax after fixed amout subtracted
    pub fn ratio(&self) -> Ratio {
        Ratio(self.0.ratio)
    }

    /// Max limit of tax
    pub fn max_limit(&self) -> Option<NonZero> {
        self.0.max_limit.map(|n| NonZero(format!("{}", n)))
    }
}

struct Ratio(chain_impl_mockchain::rewards::Ratio);

#[juniper::object(
    Context = Context,
)]
impl Ratio {
    pub fn numerator(&self) -> Value {
        Value(format!("{}", self.0.numerator))
    }

    pub fn denominator(&self) -> NonZero {
        NonZero(format!("{}", self.0.denominator))
    }
}

pub struct Proposal(certificate::Proposal);

#[juniper::object(
    Context = Context,
)]
impl Proposal {
    pub fn external_id(&self) -> ExternalProposalId {
        ExternalProposalId(self.0.external_id().to_string())
    }

    /// get the vote options range
    ///
    /// this is the available range of choices to make for the given
    /// proposal. all casted votes for this proposals ought to be in
    /// within the given range
    pub fn options(&self) -> VoteOptionRange {
        self.0.options().clone().into()
    }
}

#[derive(Clone)]
pub struct Pool {
    id: certificate::PoolId,
    data: Option<StakePoolData>,
    blocks: Option<PersistentSequence<HeaderHash>>,
}

impl Pool {
    fn from_string_id(id: &str, db: &DB) -> FieldResult<Pool> {
        let id = certificate::PoolId::from_str(&id)?;
        let blocks = block_on(db.get_stake_pool_blocks(&id))
            .ok_or_else(|| Error::NotFound("Stake pool not found".to_owned()))?;

        let data = block_on(db.get_stake_pool_data(&id))
            .ok_or_else(|| Error::NotFound("Stake pool not found".to_owned()))?;

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

    fn new_with_data(id: certificate::PoolId, data: StakePoolData) -> Self {
        Pool {
            id,
            blocks: None,
            data: Some(data),
        }
    }
}

#[juniper::object(
    Context = Context
)]
impl Pool {
    pub fn id(&self) -> PoolId {
        PoolId(format!("{}", &self.id))
    }

    pub fn blocks(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<IndexCursor>,
        after: Option<IndexCursor>,
        context: &Context,
    ) -> FieldResult<BlockConnection> {
        let blocks = match &self.blocks {
            Some(b) => b.clone(),
            None => block_on(context.db.get_stake_pool_blocks(&self.id)).ok_or_else(|| {
                Error::InternalError("Stake pool in block is not indexed".to_owned())
            })?,
        };

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

        let pagination_arguments = PaginationArguments {
            first,
            last,
            before: before.map(u32::try_from).transpose()?,
            after: after.map(u32::try_from).transpose()?,
        }
        .validate()?;

        BlockConnection::new(bounds, pagination_arguments, |range| match range {
            PaginationInterval::Empty => vec![],
            PaginationInterval::Inclusive(range) => (range.lower_bound..=range.upper_bound)
                .filter_map(|i| blocks.get(i).map(|h| (*h.as_ref(), i)))
                .collect(),
        })
    }

    pub fn registration(&self, context: &Context) -> FieldResult<PoolRegistration> {
        match &self.data {
            Some(data) => Ok(data.registration.clone().into()),
            None => block_on(context.db.get_stake_pool_data(&self.id))
                .map(|data| PoolRegistration::from(data.registration))
                .ok_or_else(|| Error::NotFound("Stake pool not found".to_owned()).into()),
        }
    }

    pub fn retirement(&self, context: &Context) -> FieldResult<Option<PoolRetirement>> {
        match &self.data {
            Some(data) => Ok(data.retirement.clone().map(PoolRetirement::from)),
            None => Ok(block_on(async {
                context
                    .db
                    .get_stake_pool_data(&self.id)
                    .await
                    .map(|data| data.retirement)
                    .and_then(|retirement| retirement.map(PoolRetirement::from))
            })),
        }
    }
}

struct Status {}

#[juniper::object(
    Context = Context
)]
impl Status {
    pub fn latest_block(&self, context: &Context) -> FieldResult<Block> {
        latest_block(context).map(|b| Block::from(&b))
    }

    pub fn fee_settings(&self, context: &Context) -> FeeSettings {
        let chain_impl_mockchain::fee::LinearFee {
            constant,
            coefficient,
            certificate,
            per_certificate_fees,
            per_vote_certificate_fees,
        } = context.db.blockchain_config.fees;

        FeeSettings {
            constant: Value(format!("{}", constant)),
            coefficient: Value(format!("{}", coefficient)),
            certificate: Value(format!("{}", certificate)),
            certificate_pool_registration: Value(format!(
                "{}",
                per_certificate_fees
                    .certificate_pool_registration
                    .map(|v| v.get())
                    .unwrap_or(certificate)
            )),
            certificate_stake_delegation: Value(format!(
                "{}",
                per_certificate_fees
                    .certificate_stake_delegation
                    .map(|v| v.get())
                    .unwrap_or(certificate)
            )),
            certificate_owner_stake_delegation: Value(format!(
                "{}",
                per_certificate_fees
                    .certificate_owner_stake_delegation
                    .map(|v| v.get())
                    .unwrap_or(certificate)
            )),
            certificate_vote_plan: Value(format!(
                "{}",
                per_vote_certificate_fees
                    .certificate_vote_plan
                    .map(|v| v.get())
                    .unwrap_or(certificate)
            )),
            certificate_vote_cast: Value(format!(
                "{}",
                per_vote_certificate_fees
                    .certificate_vote_cast
                    .map(|v| v.get())
                    .unwrap_or(certificate)
            )),
        }
    }
}

struct Treasury {
    rewards: Value,
    treasury: Value,
    treasury_tax: TaxType,
}

#[juniper::object(
    Context = Context
)]
impl Treasury {
    fn rewards(&self) -> &Value {
        &self.rewards
    }

    fn treasury(&self) -> &Value {
        &self.treasury
    }

    fn treasury_tax(&self) -> &TaxType {
        &self.treasury_tax
    }
}

#[derive(juniper::GraphQLObject)]
struct FeeSettings {
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
struct Epoch {
    id: header::Epoch,
}

impl Epoch {
    fn from_epoch_number(id: EpochNumber) -> FieldResult<Epoch> {
        Ok(Epoch { id: id.try_into()? })
    }

    fn get_epoch_data(&self, db: &DB) -> Option<EpochData> {
        block_on(db.get_epoch(self.id))
    }
}

#[juniper::object(
    Context = Context
)]
impl Epoch {
    pub fn id(&self) -> EpochNumber {
        self.id.into()
    }

    /// Get a paginated view of all the blocks in this epoch
    pub fn blocks(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<IndexCursor>,
        after: Option<IndexCursor>,
        context: &Context,
    ) -> FieldResult<Option<BlockConnection>> {
        let epoch_data = match self.get_epoch_data(&context.db) {
            Some(epoch_data) => epoch_data,
            None => return Ok(None),
        };

        let epoch_lower_bound = block_on(async {
            context
                .db
                .get_block(&epoch_data.first_block)
                .await
                .map(|block| u32::from(block.chain_length))
        })
        .expect("Epoch lower bound");

        let epoch_upper_bound = block_on(async {
            context
                .db
                .get_block(&epoch_data.last_block)
                .await
                .map(|block| u32::from(block.chain_length))
        })
        .expect("Epoch upper bound");

        let boundaries = PaginationInterval::Inclusive(InclusivePaginationInterval {
            lower_bound: 0,
            upper_bound: epoch_upper_bound
                .checked_sub(epoch_lower_bound)
                .expect("pagination upper_bound to be greater or equal than lower_bound"),
        });

        let pagination_arguments = PaginationArguments {
            first,
            last,
            before: before.map(u32::try_from).transpose()?,
            after: after.map(u32::try_from).transpose()?,
        }
        .validate()?;

        BlockConnection::new(boundaries, pagination_arguments, |range| match range {
            PaginationInterval::Empty => unreachable!("No blocks found (not even genesis)"),
            PaginationInterval::Inclusive(range) => block_on(context.db.get_block_hash_range(
                (range.lower_bound + epoch_lower_bound).into(),
                (range.upper_bound + epoch_lower_bound + 1).into(),
            ))
            .iter()
            .map(|(hash, index)| (*hash, u32::from(*index) - epoch_lower_bound))
            .collect(),
        })
        .map(Some)
    }

    pub fn first_block(&self, context: &Context) -> Option<Block> {
        self.get_epoch_data(&context.db)
            .map(|data| Block::from_valid_hash(data.first_block))
    }

    pub fn last_block(&self, context: &Context) -> Option<Block> {
        self.get_epoch_data(&context.db)
            .map(|data| Block::from_valid_hash(data.last_block))
    }

    pub fn total_blocks(&self, context: &Context) -> BlockCount {
        self.get_epoch_data(&context.db)
            .map_or(0u32.into(), |data| data.total_blocks.into())
    }
}

struct StakeDistribution {
    pools: Vec<PoolStakeDistribution>,
}

#[juniper::object(
    Context = Context,
)]
impl StakeDistribution {
    pub fn pools(&self) -> &Vec<PoolStakeDistribution> {
        &self.pools
    }
}

struct PoolStakeDistribution {
    pool: Pool,
    delegated_stake: Value,
}

#[juniper::object(
    Context = Context,
)]
impl PoolStakeDistribution {
    pub fn pool(&self) -> &Pool {
        &self.pool
    }

    pub fn delegated_stake(&self) -> &Value {
        &self.delegated_stake
    }
}

pub struct Query;

#[juniper::object(
    Context = Context,
)]
impl Query {
    fn block(id: String, context: &Context) -> FieldResult<Block> {
        Block::from_string_hash(id, &context.db)
    }

    fn block_by_chain_length(length: ChainLength, context: &Context) -> FieldResult<Option<Block>> {
        Ok(
            block_on(context.db.find_block_by_chain_length(length.try_into()?))
                .map(Block::from_valid_hash),
        )
    }

    /// query all the blocks in a paginated view
    fn all_blocks(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<IndexCursor>,
        after: Option<IndexCursor>,
        context: &Context,
    ) -> FieldResult<BlockConnection> {
        let longest_chain = latest_block(context)?.chain_length;

        let block0 = 0u32;

        let boundaries = PaginationInterval::Inclusive(InclusivePaginationInterval {
            lower_bound: block0,
            upper_bound: u32::from(longest_chain),
        });

        let pagination_arguments = PaginationArguments {
            first,
            last,
            before: before.map(u32::try_from).transpose()?,
            after: after.map(u32::try_from).transpose()?,
        }
        .validate()?;

        BlockConnection::new(boundaries, pagination_arguments, |range| match range {
            PaginationInterval::Empty => vec![],
            PaginationInterval::Inclusive(range) => {
                let a = range.lower_bound.into();
                let b = range.upper_bound.checked_add(1).unwrap().into();
                block_on(context.db.get_block_hash_range(a, b))
                    .iter_mut()
                    .map(|(hash, chain_length)| (*hash, u32::from(*chain_length)))
                    .collect()
            }
        })
    }

    fn transaction(id: String, context: &Context) -> FieldResult<Transaction> {
        let id = FragmentId::from_str(&id)?;

        Transaction::from_id(id, context)
    }

    fn epoch(id: EpochNumber, context: &Context) -> FieldResult<Epoch> {
        Epoch::from_epoch_number(id)
    }

    fn address(bech32: String, context: &Context) -> FieldResult<Address> {
        Address::from_bech32(&bech32)
    }

    pub fn stake_pool(id: PoolId, context: &Context) -> FieldResult<Pool> {
        Pool::from_string_id(&id.0, &context.db)
    }

    pub fn all_stake_pools(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<IndexCursor>,
        after: Option<IndexCursor>,
        context: &Context,
    ) -> FieldResult<PoolConnection> {
        let mut stake_pools = block_on(context.db.get_stake_pools());

        // Although it's probably not a big performance concern
        // There are a few alternatives to not have to sort this
        // - A separate data structure can be used to track InsertionOrder -> PoolId
        // (or any other order)
        // - Find some way to rely in the Hamt iterator order (but I think this is probably not a good idea)
        stake_pools.sort_unstable_by_key(|(id, data)| id.clone());

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

        let pagination_arguments = PaginationArguments {
            first,
            last,
            before: before.map(u32::try_from).transpose()?,
            after: after.map(u32::try_from).transpose()?,
        }
        .validate()?;

        PoolConnection::new(boundaries, pagination_arguments, |range| match range {
            PaginationInterval::Empty => vec![],
            PaginationInterval::Inclusive(range) => {
                let from = range.lower_bound;
                let to = range.upper_bound;

                (from..=to)
                    .map(|i: u32| {
                        let (pool_id, stake_pool_data) = &stake_pools[usize::try_from(i).unwrap()];
                        (
                            Pool::new_with_data(
                                certificate::PoolId::clone(pool_id),
                                StakePoolData::clone(stake_pool_data),
                            ),
                            i,
                        )
                    })
                    .collect::<Vec<(Pool, u32)>>()
            }
        })
    }

    pub fn status() -> FieldResult<Status> {
        Ok(Status {})
    }
}

pub struct Context {
    pub db: DB,
    pub settings: GraphQLSettings,
}

impl juniper::Context for Context {}

pub type Schema = RootNode<'static, Query, EmptyMutation<Context>>;

pub fn create_schema() -> Schema {
    Schema::new(Query {}, EmptyMutation::new())
}

fn latest_block(context: &Context) -> FieldResult<ExplorerBlock> {
    block_on(async {
        let hash = context.db.get_latest_block_hash().await;
        context.db.get_block(&hash).await
    })
    .ok_or_else(|| Error::InternalError("tip is not in explorer".to_owned()))
    .map_err(Into::into)
}

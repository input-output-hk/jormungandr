use super::ExplorerBlock;
use crate::blockcfg::{self, FragmentId, HeaderHash};
pub use juniper::http::GraphQLRequest;
use juniper::EmptyMutation;
use juniper::FieldError;
use juniper::FieldResult;
use juniper::RootNode;
use std::str::FromStr;
use tokio::prelude::*;

use crate::explorer::ExplorerDB;
use juniper::graphql_value;

pub struct Block {
    hash: String,
    date: BlockDate,
    chain_length: ChainLength,
}

/// A Block
#[juniper::object(
    Context = Context
)]
impl Block {
    /// The Block unique identifier
    pub fn hash(&self) -> &String {
        &self.hash
    }

    /// Date the Block was included in the blockchain
    pub fn date(&self) -> &BlockDate {
        &self.date
    }

    /// The transactions contained in the block
    pub fn transactions(&self) -> Vec<&Transaction> {
        unimplemented!()
    }

    pub fn previous_block(&self) -> Option<&Block> {
        unimplemented!()
    }

    pub fn next_block(&self) -> Option<&Block> {
        unimplemented!()
    }

    pub fn chain_length(&self) -> &ChainLength {
        &self.chain_length
    }
}

impl From<ExplorerBlock> for Block {
    fn from(block: ExplorerBlock) -> Block {
        Block {
            hash: block.id().to_string(),
            date: block.date().into(),
            chain_length: ChainLength(u32::from(block.chain_length()).to_string()),
        }
    }
}

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

impl From<blockcfg::BlockDate> for BlockDate {
    fn from(date: blockcfg::BlockDate) -> BlockDate {
        BlockDate {
            epoch: Epoch {
                id: EpochNumber(format!("{}", date.epoch)),
            },
            slot: Slot(format!("{}", date.slot_id)),
        }
    }
}

struct Transaction {
    id: FragmentId,
    in_block: HeaderHash,
}

impl Transaction {
    fn new(id: FragmentId, context: &Context) -> FieldResult<Option<Transaction>> {
        let in_block_option = context.db.find_block_by_transaction(&id).wait()?;
        Ok(in_block_option.map(|block_id| Transaction {
            id,
            in_block: block_id,
        }))
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
        let block_option = context
            .db
            .get_block(&self.in_block)
            .map_err(|err| FieldError::from(err))
            .wait()?;

        block_option
            .ok_or(FieldError::new(
                "Couldn't find block in explorer",
                graphql_value!({ "internal_error": "Block is not in explorer" }),
            ))
            .map(|b| b.into())
    }

    pub fn inputs(&self) -> Vec<TransactionInput> {
        unimplemented!()
    }

    pub fn outputs(&self) -> Vec<TransactionOutput> {
        unimplemented!()
    }
}

#[derive(juniper::GraphQLObject)]
struct TransactionInput {
    amount: Value,
    address: Address,
}

#[derive(juniper::GraphQLObject)]
struct TransactionOutput {
    amount: Value,
    address: Address,
}

#[derive(juniper::GraphQLObject)]
struct Address {
    delegation: StakePool,
    total_send: Value,
    total_received: Value,
}

#[derive(juniper::GraphQLObject)]
struct StakePool {
    id: PoolId,
}

struct Status {
    current_epoch: Epoch,
    latest_block: Block,
    fee: FeeSettings,
}

#[juniper::object(
    Context = Context
)]
impl Status {
    pub fn current_epoch(&self) -> &Epoch {
        &self.current_epoch
    }

    pub fn latest_block(&self) -> &Block {
        &self.latest_block
    }

    pub fn fee_settings(&self) -> &FeeSettings {
        &self.fee
    }
}

#[derive(juniper::GraphQLObject)]
struct FeeSettings {
    constant: Value,
    coefficient: Value,
    certificate: Value,
}

#[derive(juniper::GraphQLScalarValue)]
struct PoolId(String);

#[derive(juniper::GraphQLScalarValue)]
struct Value(String);

#[derive(juniper::GraphQLScalarValue)]
struct EpochNumber(String);

struct Epoch {
    id: EpochNumber,
}

#[juniper::object(
    Context = Context
)]
impl Epoch {
    pub fn id(&self) -> &EpochNumber {
        &self.id
    }

    /// Not yet implemented
    pub fn stake_distribution(&self) -> StakeDistribution {
        unimplemented!()
    }

    /// Not yet implemented
    pub fn blocks(&self) -> Vec<Block> {
        unimplemented!()
    }

    /// Not yet implemented
    pub fn total_blocks(&self) -> i32 {
        unimplemented!()
    }
}

#[derive(juniper::GraphQLObject)]
struct StakeDistribution {
    pools: Vec<PoolStakeDistribution>,
}

#[derive(juniper::GraphQLObject)]
struct PoolStakeDistribution {
    pool: StakePool,
    delegated_stake: Value,
}

#[derive(juniper::GraphQLScalarValue)]
struct Slot(String);

#[derive(juniper::GraphQLScalarValue)]
struct ChainLength(String);

pub struct Query;

#[juniper::object(
    Context = Context,
)]
impl Query {
    fn block(id: String, context: &Context) -> FieldResult<Block> {
        unimplemented!();
    }

    fn block(chain_length: ChainLength) -> FieldResult<Block> {
        unimplemented!();
    }

    fn transaction(id: String, context: &Context) -> FieldResult<Option<Transaction>> {
        // This call blocks the current thread (the call to wait), but it won't block the node's
        // thread, as queries are only executed in an exclusive runtime
        let id = FragmentId::from_str(&id)?;

        Transaction::new(id, context)
    }

    pub fn epoch(id: EpochNumber) -> FieldResult<Epoch> {
        unimplemented!();
    }

    pub fn stake_pool(id: PoolId) -> FieldResult<StakePool> {
        unimplemented!();
    }

    pub fn status() -> FieldResult<Status> {
        unimplemented!();
    }
}

pub struct Context {
    pub db: ExplorerDB,
}

impl juniper::Context for Context {}

pub type Schema = RootNode<'static, Query, EmptyMutation<Context>>;

pub fn create_schema() -> Schema {
    Schema::new(Query {}, EmptyMutation::new())
}

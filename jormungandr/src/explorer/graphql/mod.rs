mod error;
use self::error::ErrorKind;
use super::indexing::{ExplorerBlock, ExplorerTransaction};
use crate::blockcfg::{self, FragmentId, HeaderHash};
use chain_impl_mockchain::value;
pub use juniper::http::GraphQLRequest;
use juniper::EmptyMutation;
use juniper::FieldResult;
use juniper::RootNode;
use std::str::FromStr;
use tokio::prelude::*;

use crate::explorer::ExplorerDB;

pub struct Block {
    hash: HeaderHash,
    date: BlockDate,
    chain_length: ChainLength,
}

/// A Block
#[juniper::object(
    Context = Context
)]
impl Block {
    /// The Block unique identifier
    pub fn hash(&self) -> String {
        format!("{}", self.hash)
    }

    /// Date the Block was included in the blockchain
    pub fn date(&self) -> &BlockDate {
        &self.date
    }

    /// The transactions contained in the block
    pub fn transactions(&self, context: &Context) -> FieldResult<Vec<Transaction>> {
        Ok(context
            .db
            .get_block(&self.hash)
            .wait()?
            .ok_or(ErrorKind::InternalError(
                "couldn't find block in explorer db".to_owned(),
            ))?
            .transactions
            .iter()
            .map(|(id, _tx)| Transaction {
                id: id.clone(),
                in_block: self.hash.clone(),
            })
            .collect())
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

impl From<&ExplorerBlock> for Block {
    fn from(block: &ExplorerBlock) -> Block {
        Block {
            hash: block.id(),
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

    fn get_block(&self, context: &Context) -> FieldResult<ExplorerBlock> {
        context.db.get_block(&self.in_block).wait()?.ok_or(
            ErrorKind::InternalError(
                "transaction is in explorer but couldn't find its block".to_owned(),
            )
            .into(),
        )
    }

    fn get_contents(&self, context: &Context) -> FieldResult<ExplorerTransaction> {
        let block = self.get_block(context)?;
        Ok(block
            .transactions
            .get(&self.id)
            .ok_or(ErrorKind::InternalError(
                "transaction was not found in respective block".to_owned(),
            ))?
            .clone())
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

struct Address {
    id: chain_addr::Address,
}

impl From<&chain_addr::Address> for Address {
    fn from(addr: &chain_addr::Address) -> Address {
        Address { id: addr.clone() }
    }
}

#[juniper::object(
    Context = Context
)]
impl Address {
    fn id(&self) -> String {
        format!(
            "{}",
            chain_addr::AddressReadable::from_address("test", &self.id)
        )
    }

    fn delegation() -> StakePool {
        unimplemented!()
    }

    fn total_send() -> Value {
        unimplemented!()
    }

    fn total_received() -> Value {
        unimplemented!()
    }

    fn transactions() -> Vec<Transaction> {
        unimplemented!()
    }
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

impl From<&value::Value> for Value {
    fn from(v: &value::Value) -> Value {
        Value(format!("{}", v))
    }
}

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

mod error;
use self::error::ErrorKind;
use super::indexing::{EpochData, ExplorerBlock, ExplorerTransaction};
use crate::blockcfg::{self, FragmentId, HeaderHash};
use chain_impl_mockchain::value;
pub use juniper::http::GraphQLRequest;
use juniper::EmptyMutation;
use juniper::FieldResult;
use juniper::RootNode;
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;
use tokio::prelude::*;

use crate::explorer::{ExplorerDB, Settings};

pub struct Block {
    hash: HeaderHash,
}

impl Block {
    fn from_string_hash(hash: String, db: &ExplorerDB) -> FieldResult<Block> {
        let hash = HeaderHash::from_str(&hash)?;
        let block = Block { hash };

        block.get_explorer_block(db).map(|_| block)
    }

    fn from_valid_hash(hash: HeaderHash) -> Block {
        Block { hash: hash.clone() }
    }

    fn get_explorer_block(&self, db: &ExplorerDB) -> FieldResult<ExplorerBlock> {
        db.get_block(&self.hash).wait()?.ok_or(
            ErrorKind::InternalError("Couldn't find block's contents in explorer".to_owned())
                .into(),
        )
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
    pub fn transactions(&self, context: &Context) -> FieldResult<Vec<Transaction>> {
        Ok(self
            .get_explorer_block(&context.db)?
            .transactions
            .iter()
            .map(|(id, _tx)| Transaction {
                id: id.clone(),
                in_block: self.hash.clone(),
            })
            .collect())
    }

    pub fn previous_block(&self, context: &Context) -> FieldResult<Block> {
        self.get_explorer_block(&context.db)
            .map(|b| Block::from_valid_hash(b.parent_hash))
    }

    pub fn chain_length(&self, context: &Context) -> FieldResult<ChainLength> {
        self.get_explorer_block(&context.db)
            .map(|block| block.chain_length().into())
    }
}

impl From<&ExplorerBlock> for Block {
    fn from(block: &ExplorerBlock) -> Block {
        Block::from_valid_hash(block.id().clone())
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
            epoch: Epoch { id: date.epoch },
            slot: Slot(format!("{}", date.slot_id)),
        }
    }
}

struct Transaction {
    id: FragmentId,
    in_block: HeaderHash,
}

impl Transaction {
    fn from_id(id: FragmentId, context: &Context) -> FieldResult<Transaction> {
        let in_block =
            context
                .db
                .find_block_by_transaction(&id)
                .wait()?
                .ok_or(ErrorKind::NotFound(format!(
                    "transaction not found: {}",
                    &id,
                )))?;

        Ok(Transaction { id, in_block })
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

impl Address {
    fn from_bech32(bech32: &String) -> FieldResult<Address> {
        Ok(Address {
            id: chain_addr::AddressReadable::from_string_anyprefix(bech32)?.to_address(),
        })
    }
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
    /// The base32 representation of an address
    fn id(&self, context: &Context) -> String {
        chain_addr::AddressReadable::from_address(&context.settings.address_bech32_prefix, &self.id)
            .to_string()
    }

    fn delegation() -> FieldResult<StakePool> {
        Err(ErrorKind::Unimplemented.into())
    }

    fn transactions(&self, context: &Context) -> FieldResult<Vec<Transaction>> {
        let ids = context
            .db
            .get_transactions_by_address(&self.id)
            .wait()?
            .ok_or(ErrorKind::InternalError(
                "Expected address to be indexed".to_owned(),
            ))?;

        ids.iter()
            .map(|id| Transaction::from_id(id.clone(), context))
            .collect()
    }
}

#[derive(juniper::GraphQLObject)]
struct StakePool {
    id: PoolId,
}

struct Status {}

#[juniper::object(
    Context = Context
)]
impl Status {
    pub fn current_epoch(&self) -> FieldResult<Epoch> {
        // TODO: Would this be the Epoch of last block or a timeframe reference?
        Err(ErrorKind::Unimplemented.into())
    }

    pub fn latest_block(&self, context: &Context) -> FieldResult<Block> {
        context
            .db
            .get_latest_block_hash()
            .and_then(|hash| context.db.get_block(&hash))
            .wait()?
            .ok_or(ErrorKind::InternalError("tip is not in explorer".to_owned()).into())
            .map(|b| Block::from(&b))
    }

    pub fn fee_settings(&self) -> FieldResult<FeeSettings> {
        // TODO: Where can I get this?
        Err(ErrorKind::Unimplemented.into())
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

impl From<blockcfg::Epoch> for EpochNumber {
    fn from(e: blockcfg::Epoch) -> EpochNumber {
        EpochNumber(format!("{}", e))
    }
}

impl TryFrom<EpochNumber> for blockcfg::Epoch {
    type Error = std::num::ParseIntError;
    fn try_from(e: EpochNumber) -> Result<blockcfg::Epoch, Self::Error> {
        e.0.parse::<u32>()
    }
}

struct Epoch {
    id: blockcfg::Epoch,
}

impl Epoch {
    fn from_epoch_number(id: EpochNumber, db: &ExplorerDB) -> FieldResult<Epoch> {
        let epoch = Epoch { id: id.try_into()? };

        epoch.get_epoch_data(db).map(|_| epoch)
    }

    fn get_epoch_data(&self, db: &ExplorerDB) -> FieldResult<EpochData> {
        db.get_epoch(self.id.into()).wait()?.ok_or(
            ErrorKind::InternalError("Couldn't get EpochData from ExplorerDB".to_owned()).into(),
        )
    }
}

#[juniper::object(
    Context = Context
)]
impl Epoch {
    pub fn id(&self) -> EpochNumber {
        self.id.into()
    }

    /// Not yet implemented
    pub fn stake_distribution(&self) -> FieldResult<StakeDistribution> {
        Err(ErrorKind::Unimplemented.into())
    }

    /// Not yet implemented
    pub fn blocks(&self, context: &Context) -> FieldResult<Vec<Block>> {
        Err(ErrorKind::Unimplemented.into())
    }

    pub fn first_block(&self, context: &Context) -> FieldResult<Block> {
        self.get_epoch_data(&context.db)
            .map(|data| Block::from_valid_hash(data.first_block))
    }

    pub fn last_block(&self, context: &Context) -> FieldResult<Block> {
        self.get_epoch_data(&context.db)
            .map(|data| Block::from_valid_hash(data.last_block))
    }

    pub fn total_blocks(&self, context: &Context) -> FieldResult<BlockCount> {
        self.get_epoch_data(&context.db)
            .map(|data| data.total_blocks.into())
    }
}

#[derive(juniper::GraphQLScalarValue)]
struct BlockCount(String);

impl From<u32> for BlockCount {
    fn from(number: u32) -> BlockCount {
        BlockCount(format!("{}", number))
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

impl From<blockcfg::ChainLength> for ChainLength {
    fn from(length: blockcfg::ChainLength) -> ChainLength {
        ChainLength(u32::from(length).to_string())
    }
}

impl TryFrom<ChainLength> for blockcfg::ChainLength {
    type Error = std::num::ParseIntError;
    fn try_from(length: ChainLength) -> Result<blockcfg::ChainLength, Self::Error> {
        length.0.parse::<u32>().map(blockcfg::ChainLength::from)
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
        Ok(context
            .db
            .find_block_by_chain_length(length.try_into()?)
            .wait()?
            .map(Block::from_valid_hash))
    }

    fn transaction(id: String, context: &Context) -> FieldResult<Transaction> {
        let id = FragmentId::from_str(&id)?;

        Transaction::from_id(id, context)
    }

    fn epoch(id: EpochNumber, context: &Context) -> FieldResult<Epoch> {
        Epoch::from_epoch_number(id, &context.db)
    }

    fn address(bech32: String, context: &Context) -> FieldResult<Address> {
        Address::from_bech32(&bech32)
    }

    pub fn stake_pool(id: PoolId) -> FieldResult<StakePool> {
        Err(ErrorKind::Unimplemented.into())
    }

    pub fn status() -> FieldResult<Status> {
        Ok(Status {})
    }
}

pub struct Context {
    pub db: ExplorerDB,
    pub settings: Settings,
}

impl juniper::Context for Context {}

pub type Schema = RootNode<'static, Query, EmptyMutation<Context>>;

pub fn create_schema() -> Schema {
    Schema::new(Query {}, EmptyMutation::new())
}

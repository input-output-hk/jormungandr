mod error;
use self::error::ErrorKind;
use crate::blockcfg::{self, Fragment, FragmentId, Header, HeaderHash};
use crate::blockchain::Blockchain;
use crate::explorer::{self, ExplorerDB};
use chain_core::property::Fragment as _;
use chain_impl_mockchain::fee::LinearFee;
use juniper::graphql_value;
pub use juniper::http::GraphQLRequest;
use juniper::EmptyMutation;
use juniper::FieldError;
use juniper::FieldResult;
use juniper::RootNode;
use std::str::FromStr;
use tokio::prelude::*;

pub struct Block {
    header: Header,
}

impl Block {
    pub fn from_string_hash(hash: String, context: &Context) -> FieldResult<Block> {
        let hash = HeaderHash::from_str(&hash)?;
        Self::from_header_hash(hash, context)
    }

    pub fn from_header_hash(hash: HeaderHash, context: &Context) -> FieldResult<Block> {
        let header = context
            .db
            .get_header(hash)
            .wait()
            // Err = Infallible
            .unwrap()
            // None -> Missing in the explorer (not indexed)
            .ok_or(FieldError::new(
                "Couldn't find block in explorer",
                graphql_value!({ "internal_error": "Block is not in explorer" }),
            ))?;

        Ok(Block { header })
    }
}

/// A Block
#[juniper::object(
    Context = Context
)]
impl Block {
    /// The Block unique identifier
    pub fn hash(&self) -> String {
        format!("{}", &self.header.hash())
    }

    /// Date the Block was included in the blockchain
    pub fn date(&self, context: &Context) -> BlockDate {
        self.header.block_date().into()
    }

    /// The transactions contained in the block
    pub fn transactions(&self, context: &Context) -> FieldResult<Vec<Transaction>> {
        let block = context
            .blockchain
            .storage()
            .get(self.header.hash())
            .wait()?
            .ok_or(FieldError::from(ErrorKind::InternalError(
                "Transaction's block is not in storage (shouldn't happen)".to_owned(),
            )))?;

        let ids = block
            .contents
            .iter()
            .filter(|fragment| match fragment {
                Fragment::Transaction(_) => true,
                Fragment::OwnerStakeDelegation(_) => true,
                Fragment::StakeDelegation(_) => true,
                Fragment::PoolRegistration(_) => true,
                Fragment::PoolManagement(_) => true,
                _ => false,
            })
            .map(|fragment| fragment.id());

        Ok(ids.map(|id| Transaction { id }).collect())
    }

    pub fn previous_block(&self, context: &Context) -> FieldResult<Block> {
        // XXX: Check what's the parent of the Block0
        Block::from_header_hash((*self.header.block_parent_hash()).clone(), context)
    }

    pub fn next_block(&self, context: &Context) -> FieldResult<Option<Block>> {
        if let Some(header_hash) = context.db.get_next_block(self.header.hash()).wait()? {
            Ok(Some(Block::from_header_hash(header_hash, context)?))
        } else {
            Ok(None)
        }
    }

    pub fn chain_length(&self) -> ChainLength {
        self.header.chain_length().into()
    }
}

impl From<blockcfg::Block> for Block {
    fn from(block: blockcfg::Block) -> Block {
        Block {
            header: block.header,
        }
    }
}

struct BlockDate {
    epoch: blockcfg::Epoch,
    slot: Slot,
}

/// Block's date, composed of an Epoch and a Slot
#[juniper::object(
    Context = Context
)]
impl BlockDate {
    pub fn epoch(&self, context: &Context) -> FieldResult<Epoch> {
        Epoch::new(self.epoch, context)
    }

    pub fn slot(&self) -> &Slot {
        &self.slot
    }
}

impl From<&blockcfg::BlockDate> for BlockDate {
    fn from(date: &blockcfg::BlockDate) -> BlockDate {
        BlockDate {
            epoch: date.epoch,
            slot: Slot(format!("{}", date.slot_id)),
        }
    }
}

struct Transaction {
    id: FragmentId,
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
            .find_block_by_transaction(self.id)
            .map_err(|err| FieldError::from(err))
            .and_then(|hash_option| {
                future::poll_fn(move || match hash_option {
                    Some(hash) => context
                        .blockchain
                        .storage()
                        .get(hash)
                        .map_err(|err| FieldError::from(err))
                        .poll(),
                    None => Err(FieldError::new(
                        "Couldn't find transaction in explorer",
                        graphql_value!({ "internal_error": "Transaction is not in explorer" }),
                    )),
                })
            })
            .wait()?;

        block_option
            .ok_or(FieldError::new(
                "Couldn't find block in storage",
                graphql_value!({ "internal_error": "Block is not in storage" }),
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
    id: blockcfg::Epoch,
    epoch_data: explorer::EpochData,
}

impl Epoch {
    fn new(epoch_number: blockcfg::Epoch, context: &Context) -> FieldResult<Epoch> {
        context
            .db
            .get_epoch_data(epoch_number)
            .wait()?
            .map(|epoch_data| Epoch {
                id: epoch_number,
                epoch_data,
            })
            .ok_or(
                FieldError::new(
                    "Epoch is not in storage",
                    graphql_value!({ "internal_error": "Error is not in storage" }),
                )
                .into(),
            )
    }
}

#[juniper::object(
    Context = Context
)]
impl Epoch {
    pub fn id(&self) -> EpochNumber {
        EpochNumber(format!("{}", &self.id))
    }

    /// Not yet implemented
    pub fn stake_distribution(&self) -> Option<StakeDistribution> {
        unimplemented!()
    }

    /// Not yet implemented
    pub fn blocks(&self, context: &Context) -> FieldResult<Vec<Block>> {
        unimplemented!();
    }

    pub fn total_blocks(&self, context: &Context) -> FieldResult<BlockCount> {
        Ok(BlockCount(format!("{}", self.epoch_data.total_blocks)))
    }

    pub fn fee_settings(&self, context: &Context) -> FieldResult<FeeSettings> {
        Ok(self.epoch_data.fees.into())
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

    fn transaction(id: String, context: &Context) -> FieldResult<Transaction> {
        // This call blocks the current thread (the call to wait), but it won't block the node's
        // thread, as queries are only executed in an exclusive runtime
        let id = FragmentId::from_str(&id)?;

        Ok(Transaction { id })
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
    pub blockchain: Blockchain,
}

impl juniper::Context for Context {}

pub type Schema = RootNode<'static, Query, EmptyMutation<Context>>;

pub fn create_schema() -> Schema {
    Schema::new(Query {}, EmptyMutation::new())
}

mod certificates;
#[allow(dead_code)]
mod connections;
mod error;
mod scalars;

use self::connections::ConnectionFields;
use self::error::ApiError;
use self::scalars::{
    BlockCount, ChainLength, EpochNumber, ExternalProposalId, FragmentId, IndexCursor, Slot,
    TransactionCount, TransactionInputCount, TransactionOutputCount, Value, VoteOptionRange,
};
use crate::db::{self, chain_storable::BlockId, schema::Txn, ExplorerDb};
use async_graphql::connection::{Connection, EmptyFields};
use async_graphql::{
    Context, EmptyMutation, FieldResult, Object, SimpleObject, Subscription, Union,
};
use chain_impl_mockchain::block::HeaderId as HeaderHash;
use chain_impl_mockchain::certificate;
use std::sync::Arc;

pub struct Branch {
    id: db::chain_storable::BlockId,
}

#[Object]
impl Branch {
    pub async fn id(&self) -> String {
        format!("{}", self.id)
    }

    pub async fn block(&self) -> Block {
        Block {
            hash: self.id.clone(),
        }
    }

    pub async fn blocks(
        &self,
        _first: Option<i32>,
        _last: Option<i32>,
        _before: Option<String>,
        _after: Option<String>,
    ) -> FieldResult<Connection<IndexCursor, Block, ConnectionFields<BlockCount>, EmptyFields>>
    {
        Err(ApiError::Unimplemented.into())
    }

    async fn transactions_by_address(
        &self,
        _address_bech32: String,
        _first: Option<i32>,
        _last: Option<i32>,
        _before: Option<String>,
        _after: Option<String>,
    ) -> FieldResult<
        Connection<IndexCursor, Transaction, ConnectionFields<TransactionCount>, EmptyFields>,
    > {
        Err(ApiError::Unimplemented.into())
    }
}

pub struct Block {
    hash: db::chain_storable::BlockId,
}

/// A Block
#[Object]
impl Block {
    /// The Block unique identifier
    pub async fn id(&self) -> String {
        format!(
            "{}",
            chain_impl_mockchain::key::Hash::from(self.hash.clone())
        )
    }

    /// Date the Block was included in the blockchain
    pub async fn date(&self) -> FieldResult<BlockDate> {
        Err(ApiError::Unimplemented.into())
    }

    /// The transactions contained in the block
    pub async fn transactions(
        &self,
        _first: Option<i32>,
        _last: Option<i32>,
        _before: Option<String>,
        _after: Option<String>,
    ) -> FieldResult<
        Connection<IndexCursor, Transaction, ConnectionFields<TransactionCount>, EmptyFields>,
    > {
        Err(ApiError::Unimplemented.into())
    }

    pub async fn chain_length(&self) -> FieldResult<ChainLength> {
        Err(ApiError::Unimplemented.into())
    }

    pub async fn previous_block(&self) -> FieldResult<Block> {
        Err(ApiError::Unimplemented.into())
    }

    pub async fn total_input(&self) -> FieldResult<Value> {
        Err(ApiError::Unimplemented.into())
    }

    pub async fn total_output(&self) -> FieldResult<Value> {
        Err(ApiError::Unimplemented.into())
    }

    pub async fn is_confirmed(&self) -> FieldResult<bool> {
        Err(ApiError::Unimplemented.into())
    }

    pub async fn branches(&self) -> FieldResult<Vec<Branch>> {
        Err(ApiError::Unimplemented.into())
    }
}

/// Block's date, composed of an Epoch and a Slot
#[derive(Clone, SimpleObject)]
pub struct BlockDate {
    epoch: EpochNumber,
    slot: Slot,
}

impl From<db::chain_storable::BlockDate> for BlockDate {
    fn from(date: db::chain_storable::BlockDate) -> BlockDate {
        BlockDate {
            epoch: date.epoch.get().into(),
            slot: Slot(date.slot_id.get()),
        }
    }
}

#[derive(Clone)]
pub struct Transaction {
    id: db::chain_storable::FragmentId,
    block_hashes: Vec<BlockId>,
    txn: Arc<Txn>,
}

/// A transaction in the blockchain
#[Object]
impl Transaction {
    /// The hash that identifies the transaction
    pub async fn id(&self) -> String {
        format!("{}", self.id)
    }

    /// All the blocks this transaction is included in
    pub async fn blocks(&self) -> FieldResult<Vec<Block>> {
        Err(ApiError::Unimplemented.into())
    }

    pub async fn inputs(
        &self,
        _first: Option<i32>,
        _last: Option<i32>,
        _before: Option<String>,
        _after: Option<String>,
    ) -> FieldResult<
        Connection<
            IndexCursor,
            TransactionInput,
            ConnectionFields<TransactionInputCount>,
            EmptyFields,
        >,
    > {
        Err(ApiError::Unimplemented.into())
    }

    pub async fn outputs(
        &self,
        _first: Option<i32>,
        _last: Option<i32>,
        _before: Option<String>,
        _after: Option<String>,
    ) -> FieldResult<
        Connection<
            IndexCursor,
            TransactionOutput,
            ConnectionFields<TransactionOutputCount>,
            EmptyFields,
        >,
    > {
        Err(ApiError::Unimplemented.into())
    }

    pub async fn certificate(&self) -> FieldResult<Option<certificates::Certificate>> {
        Err(ApiError::Unimplemented.into())
    }
}

#[derive(Union)]
pub enum TransactionInput {
    AccountInput(AccountInput),
    UtxoInput(UtxoInput),
}

#[derive(SimpleObject)]
pub struct AccountInput {
    amount: Value,
    address: Address,
}

#[derive(SimpleObject)]
pub struct UtxoInput {
    fragment: FragmentId,
    offset: u8,
}

#[derive(SimpleObject)]
pub struct TransactionOutput {
    amount: Value,
    address: Address,
}

#[derive(Clone)]
pub struct Address {
    id: db::chain_storable::Address,
}

#[Object]
impl Address {
    /// The base32 representation of an address
    async fn id(&self, _context: &Context<'_>) -> FieldResult<String> {
        Err(ApiError::Unimplemented.into())
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

pub struct Query;

#[Object]
impl Query {
    async fn block(&self, _context: &Context<'_>, _id: String) -> FieldResult<Block> {
        Err(ApiError::Unimplemented.into())
    }

    async fn blocks_by_chain_length(
        &self,
        _context: &Context<'_>,
        _length: ChainLength,
    ) -> FieldResult<Vec<Block>> {
        Err(ApiError::Unimplemented.into())
    }

    async fn transaction(&self, _context: &Context<'_>, _id: String) -> FieldResult<Transaction> {
        Err(ApiError::Unimplemented.into())
    }

    /// get all current branch heads, sorted (descending) by their length
    pub async fn branches(&self, _context: &Context<'_>) -> FieldResult<Vec<Branch>> {
        Err(ApiError::Unimplemented.into())
    }

    /// get the state that the ledger currently considers as the main branch
    async fn tip(&self, _context: &Context<'_>) -> FieldResult<Branch> {
        Err(ApiError::Unimplemented.into())
    }

    pub async fn branch(&self, _context: &Context<'_>, _id: String) -> FieldResult<Branch> {
        Err(ApiError::Unimplemented.into())
    }

    pub async fn address(&self, _context: &Context<'_>, _bech32: String) -> FieldResult<Address> {
        Err(ApiError::Unimplemented.into())
    }
}

pub struct Subscription;

#[Subscription]
impl Subscription {
    async fn tip(&self, context: &Context<'_>) -> impl futures::Stream<Item = Branch> + '_ {
        use futures::StreamExt;
        let context = extract_context(context);

        tokio_stream::wrappers::BroadcastStream::new(context.tip_stream.subscribe())
            // missing a tip update doesn't seem that important, so I think it's
            // fine to ignore the error
            .filter_map(move |tip| async move { tip.ok().map(|id| Branch { id: id.into() }) })
    }
}

pub type Schema = async_graphql::Schema<Query, EmptyMutation, Subscription>;

pub struct EContext {
    pub db: ExplorerDb,
    pub tip_stream: tokio::sync::broadcast::Sender<HeaderHash>,
    pub settings: super::Settings,
}

fn extract_context<'a>(context: &Context<'a>) -> &'a EContext {
    context.data_unchecked::<EContext>()
}

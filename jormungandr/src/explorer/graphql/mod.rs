use crate::blockcfg::{self, FragmentId};
use crate::blockchain::Blockchain;
use chain_core::property::Block as _;
pub use juniper::http::GraphQLRequest;
use juniper::EmptyMutation;
use juniper::FieldResult;
use juniper::RootNode;
use std::str::FromStr;
use tokio::prelude::Future;

use crate::explorer::ExplorerDB;

#[derive(juniper::GraphQLObject)]
/// A Block
struct Block {
    /// The Block unique identifier
    hash: String,
    /// Date the Block was included in the blockchain
    date: BlockDate,
}

#[derive(juniper::GraphQLObject)]
/// Block's date, composed of an Epoch and a Slot
struct BlockDate {
    epoch: Epoch,
    slot: Slot,
}

impl From<blockcfg::BlockDate> for BlockDate {
    fn from(date: blockcfg::BlockDate) -> BlockDate {
        BlockDate {
            epoch: Epoch(format!("{}", date.epoch)),
            slot: Slot(format!("{}", date.slot_id)),
        }
    }
}

#[derive(juniper::GraphQLScalarValue)]
struct Epoch(String);

#[derive(juniper::GraphQLScalarValue)]
struct Slot(String);

pub struct Query;

#[juniper::object(
    Context = Context,
)]
impl Query {
    /// Query a Block from a Transaction hash
    fn block(transaction: String, context: &Context) -> FieldResult<Option<Block>> {
        // This call blocks the current thread (the call to wait), but it won't block the node's
        // thread, as queries are only executed in an exclusive runtime
        let id = FragmentId::from_str(&transaction)?;
        let block = context
            .db
            .find_block_by_transaction(id, context.blockchain.clone())
            .wait()?
            .map(|b| Block {
                hash: b.id().to_string(),
                date: b.date().into(),
            });
        Ok(block)
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

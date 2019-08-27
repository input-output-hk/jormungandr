extern crate juniper;
pub use self::juniper::http::GraphQLRequest;
use self::juniper::EmptyMutation;
use self::juniper::FieldResult;
use self::juniper::RootNode;
use crate::blockcfg::{self, FragmentId};
use crate::blockchain::Blockchain;
use chain_core::property::Block as _;
use std::str::FromStr;
use tokio::prelude::Future;

use crate::explorer::ExplorerDB;

#[derive(juniper::GraphQLObject)]
#[graphql(description = "change this")]
struct Block {
    hash: String,
    date: BlockDate,
}

#[derive(juniper::GraphQLObject)]
#[graphql(description = "block date")]
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
    fn block(transaction: String, context: &Context) -> FieldResult<Option<Block>> {
        // Warning: This call blocks the current thread
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

impl self::juniper::Context for Context {}

pub type Schema = RootNode<'static, Query, EmptyMutation<Context>>;

pub fn create_schema() -> Schema {
    Schema::new(Query {}, EmptyMutation::new())
}

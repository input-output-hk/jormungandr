use crate::blockcfg::{self, FragmentId};
use crate::blockchain::Blockchain;
use chain_core::property::Block as _;
use chain_core::property::Fragment;
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
    /// The transactions contained in the block
    transactions: Vec<Transaction>,
}

impl From<blockcfg::Block> for Block {
    fn from(block: blockcfg::Block) -> Block {
        Block {
            hash: block.id().to_string(),
            date: block.date().into(),
            transactions: block
                .contents
                .iter()
                .filter_map(|fragment| {
                    //TODO: Implement
                    None
                })
                .collect(),
        }
    }
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

#[derive(juniper::GraphQLObject)]
// A transaction in the blockchain
struct Transaction {
    // The hash that identifies the transaction
    id: String,
    // The block this transaction is in
    block: Block,
    inputs: Vec<TransactionInput>,
    outputs: Vec<TransactionOutput>,
}

#[derive(juniper::GraphQLObject)]
struct TransactionInput {
    amount: Lovelaces,
    kind: TransactionInputKind,
}

#[derive(juniper::GraphQLObject)]
struct TransactionOutput {
    amount: Lovelaces,
}

#[derive(juniper::GraphQLEnum)]
enum TransactionInputKind {
    Utxo,
    Account,
}

#[derive(juniper::GraphQLScalarValue)]
struct Lovelaces(String);

#[derive(juniper::GraphQLScalarValue)]
struct Epoch(String);

#[derive(juniper::GraphQLScalarValue)]
struct Slot(String);

pub struct Query;

#[juniper::object(
    Context = Context,
)]
impl Query {
    fn block(id: String, context: &Context) -> FieldResult<Vec<Block>> {
        unimplemented!();
    }

    fn transaction(id: String, context: &Context) -> FieldResult<Option<Transaction>> {
        // This call blocks the current thread (the call to wait), but it won't block the node's
        // thread, as queries are only executed in an exclusive runtime
        let id = FragmentId::from_str(&id)?;
        let block_option = context
            .db
            .find_block_by_transaction(id, context.blockchain.clone())
            .wait()?;

        let block = match block_option {
            Some(b) => b,
            None => return Ok(None),
        };

        let tx = block
            .contents
            .iter()
            .find(|fragment| fragment.id() == id)
            .map(|tx| tx.clone())
            // FIXME: Maybe throw some error, although this shouldn't happen
            .ok_or_else(|| unreachable!());

        tx.map(|tx| Transaction {
            id: format!("{}", id),
            block: block.into(),
            // TODO: Transform this things
            inputs: Vec::new(),
            outputs: Vec::new(),
        })
        .map(Some)
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

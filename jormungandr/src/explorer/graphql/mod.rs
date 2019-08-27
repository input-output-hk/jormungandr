extern crate juniper;
pub use self::juniper::http::GraphQLRequest;
use self::juniper::EmptyMutation;
use self::juniper::FieldResult;
use self::juniper::RootNode;

use crate::explorer::Process as Explorer;

#[derive(juniper::GraphQLObject)]
#[graphql(description = "change this")]
struct Block {
    hash: String,
    date: BlockDate,
    transactions: Vec<Transaction>,
    depth: ChainLength,
}

#[derive(juniper::GraphQLObject)]
#[graphql(description = "block date")]
struct BlockDate {
    epoch: Epoch,
    slot: Slot,
}

#[derive(juniper::GraphQLScalarValue)]
struct Epoch(String);

#[derive(juniper::GraphQLScalarValue)]
struct Slot(String);

#[derive(juniper::GraphQLScalarValue)]
struct ChainLength(String);

#[derive(juniper::GraphQLObject)]
#[graphql(description = "change this")]
struct Transaction {}

pub struct Query;

#[juniper::object(
    Context = Context,
)]
impl Query {
    fn block(id: String, context: &Context) -> FieldResult<Block> {
        Ok(Block {
            hash: "test".to_owned(),
            date: BlockDate {
                epoch: Epoch("1".to_owned()),
                slot: Slot("2".to_owned()),
            },
            transactions: Vec::new(),
            depth: ChainLength("3".to_owned()),
        })
    }
}

pub struct Context {
    explorer: Explorer,
}

impl self::juniper::Context for Context {}

pub type Schema = RootNode<'static, Query, EmptyMutation<Context>>;

pub fn create_schema() -> Schema {
    Schema::new(Query {}, EmptyMutation::new())
}

#[cfg(test)]
mod test {
    use super::juniper::graphql_value;
    use super::*;

    #[test]
    fn test_graphql() {
        let ctx = Context {
            explorer: explorer::Process::new(),
        };

        // Run the executor.
        let (res, _errors) = super::juniper::execute(
            "query { block(id: \"test\") {
                hash,
                date {
                    epoch,
                    slot,
                }
            } }",
            None,
            &Schema::new(Query, EmptyMutation::new()),
            &juniper::Variables::new(),
            &ctx,
        )
        .unwrap();

        let mut expected = graphql_value!(
            {"block" :
                {
                    "hash": "test",
                    "date": { "epoch": "1", "slot": "2" }
                }
            }
        );

        assert_eq!(res, expected);
    }
}

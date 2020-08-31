pub type Slot = String;
pub type ChainLength = String;
pub type EpochNumber = String;

use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "resources/explorer/graphql/lastblock.graphql",
    schema_path = "resources/explorer/graphql/schema.graphql",
    response_derives = "Debug"
)]
pub struct LastBlock;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "resources/explorer/graphql/transaction_by_id.graphql",
    schema_path = "resources/explorer/graphql/schema.graphql",
    response_derives = "Debug"
)]
pub struct TransactionById;

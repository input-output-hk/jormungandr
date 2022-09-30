pub type EpochNumber = String;
pub type Slot = String;
pub type ChainLength = String;
pub type PoolId = String;
pub type Value = String;
pub type VotePlanId = String;
pub type NonZero = String;
pub type TimeOffsetSeconds = String;
pub type PublicKey = String;
pub type ExternalProposalId = String;
pub type Weight = String;
use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "resources/explorer/graphql/address.graphql",
    schema_path = "resources/explorer/graphql/schema.graphql",
    response_derives = "Debug"
)]
pub struct Address;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "resources/explorer/graphql/transactions_by_address.graphql",
    schema_path = "resources/explorer/graphql/schema.graphql",
    response_derives = "Debug"
)]
pub struct TransactionsByAddress;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "resources/explorer/graphql/allblocks.graphql",
    schema_path = "resources/explorer/graphql/schema.graphql",
    response_derives = "Debug"
)]
pub struct AllBlocks;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "resources/explorer/graphql/block.graphql",
    schema_path = "resources/explorer/graphql/schema.graphql",
    response_derives = "Debug"
)]
pub struct Block;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "resources/explorer/graphql/allstakepools.graphql",
    schema_path = "resources/explorer/graphql/schema.graphql",
    response_derives = "Debug"
)]
pub struct AllStakePools;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "resources/explorer/graphql/blocksbychainlength.graphql",
    schema_path = "resources/explorer/graphql/schema.graphql",
    response_derives = "Debug"
)]
pub struct BlocksByChainLength;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "resources/explorer/graphql/epoch.graphql",
    schema_path = "resources/explorer/graphql/schema.graphql",
    response_derives = "Debug"
)]
pub struct Epoch;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "resources/explorer/graphql/lastblock.graphql",
    schema_path = "resources/explorer/graphql/schema.graphql",
    response_derives = "Debug"
)]
pub struct LastBlock;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "resources/explorer/graphql/stakepool.graphql",
    schema_path = "resources/explorer/graphql/schema.graphql",
    response_derives = "Debug"
)]
pub struct StakePool;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "resources/explorer/graphql/settings.graphql",
    schema_path = "resources/explorer/graphql/schema.graphql",
    response_derives = "Debug"
)]
pub struct Settings;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "resources/explorer/graphql/transaction_by_id.graphql",
    schema_path = "resources/explorer/graphql/schema.graphql",
    response_derives = "Debug,Clone"
)]
pub struct TransactionById;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "resources/explorer/graphql/transaction_by_id_certificates.graphql",
    schema_path = "resources/explorer/graphql/schema.graphql",
    response_derives = "Debug,Clone"
)]
pub struct TransactionByIdCertificates;

#[derive(GraphQLQuery)]
#[allow(clippy::upper_case_acronyms)]
#[graphql(
    query_path = "resources/explorer/graphql/voteplans.graphql",
    schema_path = "resources/explorer/graphql/schema.graphql",
    response_derives = "Debug"
)]
pub struct AllVotePlans;

#[derive(GraphQLQuery)]
#[allow(clippy::upper_case_acronyms)]
#[graphql(
    query_path = "resources/explorer/graphql/block_by_id.graphql",
    schema_path = "resources/explorer/graphql/schema.graphql",
    response_derives = "Debug,Clone"
)]
pub struct BlockById;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "resources/explorer/graphql/voteplan_by_id.graphql",
    schema_path = "resources/explorer/graphql/schema.graphql",
    response_derives = "Debug,Clone"
)]
pub struct VotePlanById;

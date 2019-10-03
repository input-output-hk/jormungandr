mod connections;
mod error;
mod scalars;
use self::connections::{BlockConnection, BlockCursor};
use self::error::ErrorKind;
use super::indexing::{EpochData, ExplorerBlock, ExplorerTransaction};
use crate::blockcfg::{self, FragmentId, HeaderHash};
use chain_impl_mockchain::certificate;
pub use juniper::http::GraphQLRequest;
use juniper::{graphql_union, EmptyMutation, FieldResult, RootNode};
use std::convert::TryFrom;
use std::convert::TryInto;
use std::str::FromStr;
use tokio::prelude::*;

use self::scalars::{
    BlockCount, ChainLength, EpochNumber, PoolId, PublicKey, Serial, Slot, TimeOffsetSeconds, Value,
};

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

    pub fn certificate(&self, context: &Context) -> FieldResult<Option<Certificate>> {
        let transaction = self.get_contents(context)?;
        match transaction.certificate {
            Some(c) => Certificate::try_from(c).map(Some).map_err(|e| e.into()),
            None => Ok(None),
        }
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

    fn delegation() -> FieldResult<Pool> {
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

/*--------------------------------------------*/
/*------------------Certificates-------------*/
/*------------------------------------------*/

struct StakeDelegation {
    delegation: certificate::StakeDelegation,
}

impl From<certificate::StakeDelegation> for StakeDelegation {
    fn from(delegation: certificate::StakeDelegation) -> StakeDelegation {
        StakeDelegation { delegation }
    }
}

#[juniper::object(
    Context = Context,
)]
impl StakeDelegation {
    // FIXME: Maybe a new Account type would be better?
    pub fn account(&self, context: &Context) -> FieldResult<Address> {
        let discrimination = context.db.blockchain_config.discrimination;
        self.delegation
            .account_id
            .to_single_account()
            .ok_or(
                // TODO: Multisig address?
                ErrorKind::Unimplemented.into(),
            )
            .map(|single| {
                chain_addr::Address(discrimination, chain_addr::Kind::Account(single.into()))
            })
            .map(|addr| Address::from(&addr))
    }

    pub fn pool(&self, context: &Context) -> Pool {
        Pool {
            id: PoolId(format!("{}", self.delegation.pool_id)),
        }
    }
}

#[derive(Clone)]
struct PoolRegistration {
    registration: certificate::PoolRegistration,
}

impl From<certificate::PoolRegistration> for PoolRegistration {
    fn from(registration: certificate::PoolRegistration) -> PoolRegistration {
        PoolRegistration { registration }
    }
}

#[juniper::object(
    Context = Context,
)]
impl PoolRegistration {
    pub fn pool(&self, context: &Context) -> Pool {
        Pool {
            id: PoolId(format!("{}", self.registration.to_id())),
        }
    }

    /// A random value, for user purpose similar to a UUID.
    /// it may not be unique over a blockchain, so shouldn't be used a unique identifier
    pub fn serial(&self) -> Serial {
        self.registration.serial.into()
    }

    /// Beginning of validity for this pool, this is used
    /// to keep track of the period of the expected key and the expiry
    pub fn start_validity(&self) -> TimeOffsetSeconds {
        self.registration.start_validity.into()
    }

    /// Management threshold for owners, this need to be <= #owners and > 0
    pub fn management_threshold(&self) -> i32 {
        // XXX: u16 fits in i32, but maybe some kind of custom scalar is better?
        self.registration.management_threshold.into()
    }

    /// Owners of this pool
    pub fn owners(&self) -> Vec<PublicKey> {
        self.registration
            .owners
            .iter()
            .map(PublicKey::from)
            .collect()
    }

    // TODO: rewards
    // TODO: keys
}

struct OwnerStakeDelegation {
    owner_stake_delegation: certificate::OwnerStakeDelegation,
}

impl From<certificate::OwnerStakeDelegation> for OwnerStakeDelegation {
    fn from(owner_stake_delegation: certificate::OwnerStakeDelegation) -> OwnerStakeDelegation {
        OwnerStakeDelegation {
            owner_stake_delegation,
        }
    }
}

#[juniper::object(
    Context = Context,
)]
impl OwnerStakeDelegation {
    fn pool(&self) -> Pool {
        Pool {
            id: PoolId(format!("{}", self.owner_stake_delegation.pool_id)),
        }
    }
}

enum Certificate {
    StakeDelegation(StakeDelegation),
    OwnerStakeDelegation(OwnerStakeDelegation),
    PoolRegistration(PoolRegistration),
    // TODO: PoolManagement
}

impl TryFrom<chain_impl_mockchain::certificate::Certificate> for Certificate {
    type Error = error::Error;
    fn try_from(
        original: chain_impl_mockchain::certificate::Certificate,
    ) -> Result<Certificate, Self::Error> {
        match original {
            certificate::Certificate::StakeDelegation(c) => {
                Ok(Certificate::StakeDelegation(StakeDelegation::from(c)))
            }
            certificate::Certificate::OwnerStakeDelegation(c) => Ok(
                Certificate::OwnerStakeDelegation(OwnerStakeDelegation::from(c)),
            ),
            certificate::Certificate::PoolRegistration(c) => {
                Ok(Certificate::PoolRegistration(PoolRegistration::from(c)))
            }
            certificate::Certificate::PoolManagement(_) => Err(ErrorKind::Unimplemented.into()),
        }
    }
}

graphql_union!(Certificate: Context |&self| {
    // the left hand side of the `instance_resolvers` match-like structure is the one
    // that's used to match in the graphql query with the `__typename` field
    instance_resolvers: |_| {
        &StakeDelegation => match *self { Certificate::StakeDelegation(ref c) => Some(c), _ => None },
        &OwnerStakeDelegation => match *self { Certificate::OwnerStakeDelegation(ref c) => Some(c), _ => None },
        &PoolRegistration => match *self { Certificate::PoolRegistration(ref c) => Some(c), _ => None },
    }
});

struct Pool {
    id: PoolId,
}

#[juniper::object(
    Context = Context
)]
impl Pool {
    pub fn id(&self) -> &PoolId {
        &self.id
    }
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

struct Epoch {
    id: blockcfg::Epoch,
}

impl Epoch {
    fn from_epoch_number(id: EpochNumber) -> FieldResult<Epoch> {
        Ok(Epoch { id: id.try_into()? })
    }

    fn get_epoch_data(&self, db: &ExplorerDB) -> Option<EpochData> {
        db.get_epoch(self.id.into())
            .wait()
            .expect("Infallible to not happen")
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
    // It is possible to compute this by getting the last block and going backwards
    // so this could fill another requirement, like pagination
    pub fn blocks(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<BlockCursor>,
        after: Option<BlockCursor>,
        context: &Context,
    ) -> FieldResult<Option<BlockConnection>> {
        let epoch_data = match self.get_epoch_data(&context.db) {
            Some(epoch_data) => epoch_data,
            None => return Ok(None),
        };

        let lower_bound = context
            .db
            .get_block(&epoch_data.first_block)
            .map(|block| BlockCursor::from(block.expect("The block to be indexed").chain_length))
            .wait()?;

        let upper_bound = context
            .db
            .get_block(&epoch_data.last_block)
            .map(|block| BlockCursor::from(block.expect("The block to be indexed").chain_length))
            .wait()?;

        BlockConnection::new(
            lower_bound,
            upper_bound,
            first,
            last,
            before,
            after,
            &context.db,
        )
        .map(Some)
    }

    pub fn first_block(&self, context: &Context) -> Option<Block> {
        self.get_epoch_data(&context.db)
            .map(|data| Block::from_valid_hash(data.first_block))
    }

    pub fn last_block(&self, context: &Context) -> Option<Block> {
        self.get_epoch_data(&context.db)
            .map(|data| Block::from_valid_hash(data.last_block))
    }

    pub fn total_blocks(&self, context: &Context) -> BlockCount {
        self.get_epoch_data(&context.db)
            .map_or(0.into(), |data| data.total_blocks.into())
    }
}

struct StakeDistribution {
    pools: Vec<PoolStakeDistribution>,
}

#[juniper::object(
    Context = Context,
)]
impl StakeDistribution {
    pub fn pools(&self) -> &Vec<PoolStakeDistribution> {
        &self.pools
    }
}

struct PoolStakeDistribution {
    pool: Pool,
    delegated_stake: Value,
}

#[juniper::object(
    Context = Context,
)]
impl PoolStakeDistribution {
    pub fn pool(&self) -> &Pool {
        &self.pool
    }

    pub fn delegated_stake(&self) -> &Value {
        &self.delegated_stake
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

    /// query all the blocks in a paginated view
    fn all_blocks(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<BlockCursor>,
        after: Option<BlockCursor>,
        context: &Context,
    ) -> FieldResult<BlockConnection> {
        let longest_chain = context
            .db
            .get_latest_block_hash()
            .and_then(|hash| context.db.get_block(&hash))
            .wait()?
            .ok_or(ErrorKind::InternalError(
                "tip is not in explorer".to_owned(),
            ))
            .map(|block| block.chain_length)?;

        let block0 = blockcfg::ChainLength::from(0u32);

        BlockConnection::new(
            block0.into(),
            longest_chain.into(),
            first,
            last,
            before,
            after,
            &context.db,
        )
    }

    fn transaction(id: String, context: &Context) -> FieldResult<Transaction> {
        let id = FragmentId::from_str(&id)?;

        Transaction::from_id(id, context)
    }

    fn epoch(id: EpochNumber, context: &Context) -> FieldResult<Epoch> {
        Epoch::from_epoch_number(id)
    }

    fn address(bech32: String, context: &Context) -> FieldResult<Address> {
        Address::from_bech32(&bech32)
    }

    pub fn stake_pool(id: PoolId) -> FieldResult<Pool> {
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

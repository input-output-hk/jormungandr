mod connections;
mod error;
mod scalars;
use self::connections::{
    BlockConnection, InclusivePaginationInterval, PaginationArguments, PaginationInterval,
    TransactionConnection, TransactionNodeFetchInfo,
};
use self::error::ErrorKind;
use super::indexing::{
    BlockProducer, EpochData, ExplorerAddress, ExplorerBlock, ExplorerTransaction, StakePoolData,
};
use super::persistent_sequence::PersistentSequence;
use crate::blockcfg::{self, FragmentId, HeaderHash};
use cardano_legacy_address::Addr as OldAddress;
use chain_impl_mockchain::certificate;
use chain_impl_mockchain::leadership::bft;
pub use juniper::http::GraphQLRequest;
use juniper::{graphql_union, EmptyMutation, FieldResult, RootNode};
use std::convert::TryFrom;
use std::convert::TryInto;
use std::str::FromStr;
use tokio::prelude::*;

use self::scalars::{
    BlockCount, ChainLength, EpochNumber, IndexCursor, NonZero, PoolId, PublicKey, Serial, Slot,
    TimeOffsetSeconds, Value,
};

use crate::explorer::{ExplorerDB, Settings};

#[derive(Clone)]
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
    pub fn transactions(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<IndexCursor>,
        after: Option<IndexCursor>,
        context: &Context,
    ) -> FieldResult<TransactionConnection> {
        let explorer_block = self.get_explorer_block(&context.db)?;
        let mut transactions: Vec<&ExplorerTransaction> =
            explorer_block.transactions.values().collect();

        // TODO: This may be expensive at some point, but I can't rely in
        // the HashMap's order (also, I'm assuming the order in the block matters)
        transactions
            .as_mut_slice()
            .sort_unstable_by_key(|tx| tx.offset_in_block);

        let pagination_arguments = PaginationArguments {
            first,
            last,
            before: before.map(u32::try_from).transpose()?,
            after: after.map(u32::try_from).transpose()?,
        }
        .validate()?;

        let boundaries = if transactions.len() > 0 {
            PaginationInterval::Inclusive(InclusivePaginationInterval {
                lower_bound: 0u32,
                upper_bound: transactions
                    .len()
                    .checked_sub(1)
                    .unwrap()
                    .try_into()
                    .expect("tried to paginate more than 2^32 elements"),
            })
        } else {
            PaginationInterval::Empty
        };

        TransactionConnection::new(
            boundaries,
            pagination_arguments,
            |range: PaginationInterval<u32>| match range {
                PaginationInterval::Empty => vec![],
                PaginationInterval::Inclusive(range) => {
                    let from = usize::try_from(range.lower_bound).unwrap();
                    let to = usize::try_from(range.upper_bound).unwrap();

                    (from..=to)
                        .map(|i| {
                            (
                                TransactionNodeFetchInfo::Contents(transactions[i].clone()),
                                i.try_into().unwrap(),
                            )
                        })
                        .collect::<Vec<(TransactionNodeFetchInfo, u32)>>()
                }
            },
        )
    }

    pub fn previous_block(&self, context: &Context) -> FieldResult<Block> {
        self.get_explorer_block(&context.db)
            .map(|b| Block::from_valid_hash(b.parent_hash))
    }

    pub fn chain_length(&self, context: &Context) -> FieldResult<ChainLength> {
        self.get_explorer_block(&context.db)
            .map(|block| block.chain_length().into())
    }

    pub fn leader(&self, context: &Context) -> FieldResult<Option<Leader>> {
        self.get_explorer_block(&context.db)
            .map(|block| match block.producer() {
                BlockProducer::StakePool(pool) => {
                    Some(Leader::StakePool(Pool::from_valid_id(pool.clone())))
                }
                BlockProducer::BftLeader(id) => {
                    Some(Leader::BftLeader(BftLeader { id: id.clone() }))
                }
                BlockProducer::None => None,
            })
    }
}

struct BftLeader {
    id: bft::LeaderId,
}

#[juniper::object(
    Context = Context,
)]
impl BftLeader {
    // FIXME: Don't use String
    fn id(&self) -> String {
        // FIXME: How to print this
        let id = &self.id;
        unimplemented!()
    }
}

enum Leader {
    StakePool(Pool),
    BftLeader(BftLeader),
}

graphql_union!(Leader: Context |&self| {
    instance_resolvers: |_| {
        &Pool => match *self { Leader::StakePool(ref c) => Some(c), _ => None },
        &BftLeader => match *self { Leader::BftLeader(ref c) => Some(c), _ => None },
    }
});

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

#[derive(Clone)]
struct Transaction {
    id: FragmentId,
    block_hash: Option<HeaderHash>,
    contents: Option<ExplorerTransaction>,
}

impl Transaction {
    fn from_id(id: FragmentId, context: &Context) -> FieldResult<Transaction> {
        let block_hash = context
            .db
            .find_block_hash_by_transaction(&id)
            .wait()
            .unwrap()
            .ok_or(ErrorKind::NotFound(format!(
                "transaction not found: {}",
                &id,
            )))?;

        Ok(Transaction {
            id,
            block_hash: Some(block_hash),
            contents: None,
        })
    }

    fn from_valid_id(id: FragmentId) -> Transaction {
        Transaction {
            id,
            block_hash: None,
            contents: None,
        }
    }

    fn from_contents(contents: ExplorerTransaction) -> Transaction {
        Transaction {
            id: contents.id,
            block_hash: None,
            contents: Some(contents),
        }
    }

    fn get_block(&self, context: &Context) -> FieldResult<ExplorerBlock> {
        let block_id = match self.block_hash {
            Some(block_id) => block_id,
            None => context
                .db
                .find_block_hash_by_transaction(&self.id)
                .wait()
                .unwrap()
                .ok_or(ErrorKind::InternalError(
                    "Transaction's block was not found".to_owned(),
                ))?,
        };

        context.db.get_block(&block_id).wait()?.ok_or(
            ErrorKind::InternalError(
                "transaction is in explorer but couldn't find its block".to_owned(),
            )
            .into(),
        )
    }

    fn get_contents(&self, context: &Context) -> FieldResult<ExplorerTransaction> {
        if let Some(c) = &self.contents {
            Ok(c.clone())
        } else {
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
    id: ExplorerAddress,
}

impl Address {
    fn from_bech32(bech32: &String) -> FieldResult<Address> {
        let addr = chain_addr::AddressReadable::from_string_anyprefix(bech32)
            .map(|adr| ExplorerAddress::New(adr.to_address()))
            .or_else(|_| OldAddress::from_str(bech32).map(|a| ExplorerAddress::Old(a)))
            .map_err(|_| ErrorKind::InvalidAddress(bech32.clone()))?;

        Ok(Address { id: addr })
    }
}

impl From<&ExplorerAddress> for Address {
    fn from(addr: &ExplorerAddress) -> Address {
        Address { id: addr.clone() }
    }
}

#[juniper::object(
    Context = Context
)]
impl Address {
    /// The base32 representation of an address
    fn id(&self, context: &Context) -> String {
        match &self.id {
            ExplorerAddress::New(addr) => chain_addr::AddressReadable::from_address(
                &context.settings.address_bech32_prefix,
                addr,
            )
            .to_string(),
            ExplorerAddress::Old(addr) => format!("{}", addr),
        }
    }

    fn delegation() -> FieldResult<Pool> {
        Err(ErrorKind::Unimplemented.into())
    }

    fn transactions(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<IndexCursor>,
        after: Option<IndexCursor>,
        context: &Context,
    ) -> FieldResult<TransactionConnection> {
        let transactions = context
            .db
            .get_transactions_by_address(&self.id)
            .wait()?
            .ok_or(ErrorKind::InternalError(
                "Expected address to be indexed".to_owned(),
            ))?;

        let boundaries = if transactions.len() > 0 {
            PaginationInterval::Inclusive(InclusivePaginationInterval {
                lower_bound: 0u64,
                upper_bound: transactions.len(),
            })
        } else {
            PaginationInterval::Empty
        };

        let pagination_arguments = PaginationArguments {
            first,
            last,
            before: before.map(u64::from),
            after: after.map(u64::from),
        }
        .validate()?;

        TransactionConnection::new(
            boundaries,
            pagination_arguments,
            |range: PaginationInterval<u64>| match range {
                PaginationInterval::Empty => vec![],
                PaginationInterval::Inclusive(range) => (range.lower_bound..=range.upper_bound)
                    .filter_map(|i| {
                        transactions
                            .get(i)
                            .map(|h| (TransactionNodeFetchInfo::Id((*h).clone()), i.into()))
                    })
                    .collect(),
            },
        )
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
            .map(|addr| Address::from(&ExplorerAddress::New(addr)))
    }

    pub fn pool(&self, context: &Context) -> Vec<Pool> {
        use chain_impl_mockchain::account::DelegationType;
        use std::iter::FromIterator as _;

        match self.delegation.get_delegation_type() {
            DelegationType::NonDelegated => vec![],
            DelegationType::Full(id) => vec![Pool::from_valid_id(id.clone())],
            DelegationType::Ratio(delegation_ratio) => Vec::from_iter(
                delegation_ratio
                    .pools()
                    .iter()
                    .cloned()
                    .map(|(p, _)| Pool::from_valid_id(p)),
            ),
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
        Pool::from_valid_id(self.registration.to_id())
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
        // XXX: u8 fits in i32, but maybe some kind of custom scalar is better?
        self.registration.management_threshold().into()
    }

    /// Owners of this pool
    pub fn owners(&self) -> Vec<PublicKey> {
        self.registration
            .owners
            .iter()
            .map(PublicKey::from)
            .collect()
    }

    pub fn operators(&self) -> Vec<PublicKey> {
        self.registration
            .operators
            .iter()
            .map(PublicKey::from)
            .collect()
    }

    pub fn rewards(&self) -> TaxType {
        TaxType(self.registration.rewards)
    }

    /// Reward account
    pub fn reward_account(&self, context: &Context) -> Option<String> {
        // TODO: What's the best way to show this? As an Address?
        self.registration
            .reward_account
            .clone()
            .map(|acc_id| format!("{:#?}", &acc_id))
    }

    // Genesis Praos keys
    // pub keys: GenesisPraosLeader,
}

struct TaxType(chain_impl_mockchain::rewards::TaxType);

#[juniper::object(
    Context = Context,
)]
impl TaxType {
    /// what get subtracted as fixed value
    pub fn fixed(&self) -> Value {
        Value(format!("{}", self.0.fixed))
    }
    /// Ratio of tax after fixed amout subtracted
    pub fn ratio(&self) -> Ratio {
        Ratio(self.0.ratio)
    }

    /// Max limit of tax
    pub fn max_limit(&self) -> Option<NonZero> {
        self.0.max_limit.map(|n| NonZero(format!("{}", n)))
    }
}

struct Ratio(chain_impl_mockchain::rewards::Ratio);
#[juniper::object(
    Context = Context,
)]
impl Ratio {
    pub fn numerator(&self) -> Value {
        Value(format!("{}", self.0.numerator))
    }

    pub fn denominator(&self) -> NonZero {
        NonZero(format!("{}", self.0.denominator))
    }
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
    fn pool(&self) -> Vec<Pool> {
        use chain_impl_mockchain::account::DelegationType;
        use std::iter::FromIterator as _;

        match self.owner_stake_delegation.get_delegation_type() {
            DelegationType::NonDelegated => vec![],
            DelegationType::Full(id) => vec![Pool::from_valid_id(id.clone())],
            DelegationType::Ratio(delegation_ratio) => Vec::from_iter(
                delegation_ratio
                    .pools()
                    .iter()
                    .cloned()
                    .map(|(p, _)| Pool::from_valid_id(p)),
            ),
        }
    }
}

// TODO can we use jormungandr-lib Certificate ?
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
            certificate::Certificate::PoolRetirement(_) => Err(ErrorKind::Unimplemented.into()),
            certificate::Certificate::PoolUpdate(_) => Err(ErrorKind::Unimplemented.into()),
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
    id: certificate::PoolId,
    data: Option<StakePoolData>,
    blocks: Option<PersistentSequence<HeaderHash>>,
}

impl Pool {
    fn from_string_id(id: &String, db: &ExplorerDB) -> FieldResult<Pool> {
        let id = certificate::PoolId::from_str(&id)?;
        let blocks = db
            .get_stake_pool_blocks(&id)
            .wait()
            .unwrap()
            .ok_or(ErrorKind::NotFound("Stake pool not found".to_owned()))?;

        let data = db
            .get_stake_pool_data(&id)
            .wait()
            .unwrap()
            .ok_or(ErrorKind::NotFound("Stake pool not found".to_owned()))?;

        Ok(Pool {
            id,
            data: Some(data),
            blocks: Some(blocks),
        })
    }

    fn from_valid_id(id: certificate::PoolId) -> Pool {
        Pool {
            id,
            blocks: None,
            data: None,
        }
    }
}

#[juniper::object(
    Context = Context
)]
impl Pool {
    pub fn id(&self) -> PoolId {
        PoolId(format!("{}", &self.id))
    }

    pub fn blocks(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<IndexCursor>,
        after: Option<IndexCursor>,
        context: &Context,
    ) -> FieldResult<BlockConnection> {
        let blocks = match &self.blocks {
            Some(b) => b.clone(),
            None => context
                .db
                .get_stake_pool_blocks(&self.id)
                .wait()
                .unwrap()
                .ok_or(ErrorKind::InternalError(
                    "Stake pool in block is not indexed".to_owned(),
                ))?,
        };

        let bounds = if blocks.len() > 0 {
            PaginationInterval::Inclusive(InclusivePaginationInterval {
                lower_bound: 0u32,
                upper_bound: blocks
                    .len()
                    .try_into()
                    .expect("Tried to paginate more than 2^32 blocks"),
            })
        } else {
            PaginationInterval::Empty
        };

        let pagination_arguments = PaginationArguments {
            first,
            last,
            before: before.map(u32::try_from).transpose()?,
            after: after.map(u32::try_from).transpose()?,
        }
        .validate()?;

        BlockConnection::new(bounds, pagination_arguments, |range| match range {
            PaginationInterval::Empty => vec![],
            PaginationInterval::Inclusive(range) => (range.lower_bound..=range.upper_bound)
                .filter_map(|i| blocks.get(i).map(|h| ((*h).clone(), i)))
                .collect(),
        })
    }

    pub fn registration(&self, context: &Context) -> FieldResult<PoolRegistration> {
        match &self.data {
            Some(data) => Ok(data.registration.clone().into()),
            None => context
                .db
                .get_stake_pool_data(&self.id)
                .wait()
                .unwrap()
                .map(|data| PoolRegistration::from(data.registration.clone()))
                .ok_or(ErrorKind::NotFound("Stake pool not found".to_owned()).into()),
        }
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

    /// Get a paginated view of all the blocks in this epoch
    pub fn blocks(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<IndexCursor>,
        after: Option<IndexCursor>,
        context: &Context,
    ) -> FieldResult<Option<BlockConnection>> {
        let epoch_data = match self.get_epoch_data(&context.db) {
            Some(epoch_data) => epoch_data,
            None => return Ok(None),
        };

        let epoch_lower_bound = context
            .db
            .get_block(&epoch_data.first_block)
            .map(|block| u32::from(block.expect("The block to be indexed").chain_length))
            .wait()?;

        let epoch_upper_bound = context
            .db
            .get_block(&epoch_data.last_block)
            .map(|block| u32::from(block.expect("The block to be indexed").chain_length))
            .wait()?;

        let boundaries = PaginationInterval::Inclusive(InclusivePaginationInterval {
            lower_bound: 0,
            upper_bound: epoch_upper_bound
                .checked_sub(epoch_lower_bound)
                .expect("pagination upper_bound to be greater or equal than lower_bound"),
        });

        let pagination_arguments = PaginationArguments {
            first,
            last,
            before: before.map(u32::try_from).transpose()?,
            after: after.map(u32::try_from).transpose()?,
        }
        .validate()?;

        BlockConnection::new(boundaries, pagination_arguments, |range| match range {
            PaginationInterval::Empty => unreachable!("No blocks found (not even genesis)"),
            PaginationInterval::Inclusive(range) => context
                .db
                .get_block_hash_range(
                    (range.lower_bound + epoch_lower_bound).into(),
                    (range.upper_bound + epoch_lower_bound + 1).into(),
                )
                .wait()
                // Error = Infallible
                .unwrap()
                .iter()
                .map(|(hash, index)| (hash.clone(), u32::from(index.clone()) - epoch_lower_bound))
                .collect(),
        })
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
            .map_or(0u32.into(), |data| data.total_blocks.into())
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
        before: Option<IndexCursor>,
        after: Option<IndexCursor>,
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

        let block0 = 0u32;

        let boundaries = PaginationInterval::Inclusive(InclusivePaginationInterval {
            lower_bound: block0,
            upper_bound: u32::from(longest_chain),
        });

        let pagination_arguments = PaginationArguments {
            first,
            last,
            before: before.map(u32::try_from).transpose()?,
            after: after.map(u32::try_from).transpose()?,
        }
        .validate()?;

        BlockConnection::new(boundaries, pagination_arguments, |range| match range {
            PaginationInterval::Empty => vec![],
            PaginationInterval::Inclusive(range) => {
                let a = range.lower_bound.into();
                let b = range.upper_bound.checked_add(1).unwrap().into();
                context
                    .db
                    .get_block_hash_range(a, b)
                    .wait()
                    .unwrap()
                    .iter_mut()
                    .map(|(hash, chain_length)| (hash.clone(), u32::from(*chain_length)))
                    .collect()
            }
        })
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

    pub fn stake_pool(id: PoolId, context: &Context) -> FieldResult<Pool> {
        Pool::from_string_id(&id.0, &context.db)
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

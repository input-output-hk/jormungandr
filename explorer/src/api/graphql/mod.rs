mod certificates;
mod connections;
mod error;
mod scalars;

use self::connections::{
    compute_interval, ConnectionFields, InclusivePaginationInterval, PaginationInterval,
    ValidatedPaginationArguments,
};
use self::error::ApiError;
use self::{
    certificates::{PrivateVoteCastCertificate, PublicVoteCastCertificate, VotePlanCertificate},
    scalars::{
        BlockCount, ChainLength, EpochNumber, ExternalProposalId, FragmentId, IndexCursor, Slot,
        TransactionCount, TransactionInputCount, TransactionOutputCount, Value, VoteOptionRange,
    },
};
use crate::db::{
    self,
    chain_storable::{BlockId, CertificateTag},
    schema::{BlockMeta, Txn},
    ExplorerDb,
};
use async_graphql::{
    connection::{query, Connection, Edge, EmptyFields},
    FieldError,
};
use async_graphql::{
    Context, EmptyMutation, FieldResult, Object, SimpleObject, Subscription, Union,
};
use chain_impl_mockchain::certificate;
use chain_impl_mockchain::{block::HeaderId as HeaderHash, transaction};
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Branch {
    id: db::chain_storable::BlockId,
    txn: Arc<Txn>,
}

#[Object]
impl Branch {
    pub async fn id(&self) -> String {
        format!("{}", self.id)
    }

    pub async fn block(&self) -> Block {
        Block {
            hash: self.id.clone(),
            txn: Arc::clone(&self.txn),
            block_meta: Mutex::new(None),
        }
    }

    pub async fn blocks(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<String>,
        after: Option<String>,
    ) -> FieldResult<Connection<IndexCursor, Block, ConnectionFields<BlockCount>, EmptyFields>>
    {
        let id = self.id.clone();
        let txn = Arc::clone(&self.txn.clone());

        query(
            after,
            before,
            first,
            last,
            move |after, before, first, last| async move {
                tokio::task::spawn_blocking(move || {
                    let connection =
                        match txn.get_blocks(&id).map_err(|_| ApiError::InternalDbError)? {
                            Some(blocks) => get_page::<u32, _, _, _, _, _, _, _>(
                                blocks,
                                EdgeOrder::Decreasing,
                                first,
                                last,
                                before,
                                after,
                                |chain_length, block_id| {
                                    Edge::new(
                                        IndexCursor::from(chain_length.get()),
                                        Block::from_valid_hash(block_id.clone(), Arc::clone(&txn)),
                                    )
                                },
                            )?,
                            // TODO: this can't really happen
                            None => Connection::with_additional_fields(
                                false,
                                false,
                                ConnectionFields { total_count: 0 },
                            ),
                        };

                    Ok::<
                        Connection<IndexCursor, Block, ConnectionFields<BlockCount>, EmptyFields>,
                        async_graphql::Error,
                    >(connection)
                })
                .await
                .unwrap()
            },
        )
        .await
    }

    async fn transactions_by_address(
        &self,
        address_bech32: String,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<String>,
        after: Option<String>,
    ) -> FieldResult<
        Connection<IndexCursor, Transaction, ConnectionFields<TransactionCount>, EmptyFields>,
    > {
        let address = chain_addr::AddressReadable::from_string_anyprefix(&address_bech32)
            .map_err(|_| ApiError::InvalidAddress(address_bech32.to_string()))?
            .to_address();

        let id = self.id.clone();

        let txn = Arc::clone(&self.txn.clone());

        query(
            after,
            before,
            first,
            last,
            move |after, before, first, last| async move {
                tokio::task::spawn_blocking(move || {
                    let connection = match txn
                        .get_transactions_by_address(&id, &address.into())
                        .map_err(|_| ApiError::InternalDbError)?
                    {
                        Some(txs) => get_page::<u64, _, _, _, _, _, _, _>(
                            txs,
                            EdgeOrder::Decreasing,
                            first,
                            last,
                            before,
                            after,
                            |offset, id| {
                                Edge::new(
                                    IndexCursor::from(offset),
                                    Transaction {
                                        id: id.clone(),
                                        block_hashes: vec![],
                                        txn: Arc::clone(&txn),
                                    },
                                )
                            },
                        )?,
                        None => Connection::with_additional_fields(
                            false,
                            false,
                            ConnectionFields { total_count: 0 },
                        ),
                    };

                    Ok::<
                        Connection<
                            IndexCursor,
                            Transaction,
                            ConnectionFields<TransactionCount>,
                            EmptyFields,
                        >,
                        async_graphql::Error,
                    >(connection)
                })
                .await
                .unwrap()
            },
        )
        .await
    }
}

pub struct Block {
    hash: db::chain_storable::BlockId,
    txn: Arc<Txn>,
    block_meta: Mutex<Option<BlockMeta>>,
}

impl Block {
    async fn from_string_hash(hash: String, txn: Arc<Txn>) -> FieldResult<Block> {
        let hash: db::chain_storable::BlockId = HeaderHash::from_str(&hash)?.into();

        if let Some(block_meta) = Self::try_get_block_meta(hash.clone(), &txn).await? {
            let block = Block {
                hash,
                txn,
                block_meta: Mutex::new(Some(block_meta)),
            };
            Ok(block)
        } else {
            Err(ApiError::NotFound(format!("block: {}", &hash,)).into())
        }
    }

    fn from_valid_hash(hash: db::chain_storable::BlockId, txn: Arc<Txn>) -> Block {
        Block {
            hash,
            txn,
            block_meta: Mutex::new(None),
        }
    }

    async fn try_get_block_meta(id: BlockId, txn: &Arc<Txn>) -> FieldResult<Option<BlockMeta>> {
        let txn = Arc::clone(&txn);
        Ok(tokio::task::spawn_blocking(move || {
            txn.get_block_meta(&id).map(|option| option.cloned())
        })
        .await
        .unwrap()?)
    }

    async fn get_block_meta(&self) -> FieldResult<BlockMeta> {
        // TODO: do proper (transparent?) async lazy initialization
        let mut guard = self.block_meta.lock().await;

        if let Some(block_meta) = &*guard {
            return Ok(block_meta.clone());
        }

        let block_meta = Self::try_get_block_meta(self.hash.clone(), &self.txn)
            .await?
            .ok_or(ApiError::InternalDbError)?;

        *guard = Some(block_meta.clone());

        Ok(block_meta)
    }
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
        Ok(self.get_block_meta().await?.date.into())
    }

    /// The transactions contained in the block
    pub async fn transactions(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<String>,
        after: Option<String>,
    ) -> FieldResult<
        Connection<IndexCursor, Transaction, ConnectionFields<TransactionCount>, EmptyFields>,
    > {
        let id = self.hash.clone();
        let txn = Arc::clone(&self.txn);
        query(
            after,
            before,
            first,
            last,
            move |after, before, first, last| async move {
                tokio::task::spawn_blocking(move || {
                    let mut txs = txn
                        .get_block_fragments(&id)
                        .map_err(|_| ApiError::InternalDbError)?;

                    let boundaries = txs
                        .first_cursor()
                        .map(|first| {
                            PaginationInterval::Inclusive(InclusivePaginationInterval {
                                lower_bound: u64::from(*first),
                                upper_bound: u64::from(*txs.last_cursor().unwrap()),
                            })
                        })
                        .unwrap_or(PaginationInterval::Empty);

                    let pagination_arguments = ValidatedPaginationArguments {
                        first,
                        last,
                        before: before.map(TryInto::try_into).transpose()?,
                        after: after.map(TryInto::try_into).transpose()?,
                    };

                    let (range, page_meta) = compute_interval(boundaries, pagination_arguments)?;

                    let mut connection = Connection::with_additional_fields(
                        page_meta.has_previous_page,
                        page_meta.has_next_page,
                        ConnectionFields {
                            total_count: page_meta.total_count,
                        },
                    );

                    match range {
                        PaginationInterval::Empty => (),
                        PaginationInterval::Inclusive(range) => {
                            let a = u8::try_from(range.lower_bound).unwrap();
                            let b = range.upper_bound;

                            txs.seek(a).map_err(|_| ApiError::InternalDbError)?;

                            // TODO: don't unwrap
                            connection.append(
                                txs.map(|i| i.unwrap())
                                    .take_while(|(h, _)| (*h as u64) <= b)
                                    .map(|(h, id)| {
                                        Edge::new(
                                            IndexCursor::from(h as u32),
                                            Transaction {
                                                id: id.clone(),
                                                block_hashes: vec![],
                                                txn: Arc::clone(&txn),
                                            },
                                        )
                                    }),
                            );
                        }
                    };

                    Ok::<
                        Connection<
                            IndexCursor,
                            Transaction,
                            ConnectionFields<TransactionCount>,
                            EmptyFields,
                        >,
                        async_graphql::Error,
                    >(connection)
                })
                .await
                .unwrap()
            },
        )
        .await
    }

    pub async fn chain_length(&self) -> FieldResult<ChainLength> {
        let id = self.hash.clone();
        let txn = Arc::clone(&self.txn);
        let chain_length = tokio::task::spawn_blocking(move || {
            txn.get_block_meta(&id)
                .map(|meta| ChainLength(meta.unwrap().chain_length.into()))
        })
        .await?
        .unwrap();

        Ok(chain_length)
    }

    pub async fn previous_block(&self) -> FieldResult<Block> {
        Ok(Block {
            hash: self.get_block_meta().await?.parent_hash,
            txn: Arc::clone(&self.txn),
            block_meta: Mutex::new(None),
        })
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
        Ok(self
            .block_hashes
            .iter()
            .cloned()
            .map(|hash| Block {
                hash,
                txn: Arc::clone(&self.txn),
                block_meta: Mutex::new(None),
            })
            .collect())
    }

    pub async fn inputs(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<String>,
        after: Option<String>,
    ) -> FieldResult<
        Connection<
            IndexCursor,
            TransactionInput,
            ConnectionFields<TransactionInputCount>,
            EmptyFields,
        >,
    > {
        let id = self.id.clone();
        let txn = Arc::clone(&self.txn);
        query(
            after,
            before,
            first,
            last,
            move |after, before, first, last| async move {
                tokio::task::spawn_blocking(move || {
                    let inputs = txn
                        .get_fragment_inputs(&id)
                        .map_err(|_| ApiError::InternalDbError)?;

                    let connection = get_page::<u8, _, _, _, _, _, _, _>(
                        inputs,
                        EdgeOrder::Increasing,
                        first,
                        last,
                        before,
                        after,
                        |h, input| {
                            let single_account = matches!(
                                input.input_type(),
                                db::chain_storable::InputType::AccountSingle
                            );

                            Edge::new(
                                IndexCursor::from(h as u32),
                                match transaction::Input::from(input).to_enum() {
                                    transaction::InputEnum::AccountInput(account_id, value) => {
                                        let address = if single_account {
                                            let public_key =
                                                account_id.to_single_account().unwrap();
                                            let kind = chain_addr::Kind::Single(public_key.into());

                                            chain_addr::Address(
                                                chain_addr::Discrimination::Test,
                                                kind,
                                            )
                                        } else {
                                            let kind = chain_addr::Kind::Multisig(
                                                account_id.to_multi_account().into(),
                                            );

                                            chain_addr::Address(
                                                chain_addr::Discrimination::Test,
                                                kind,
                                            )
                                        };
                                        TransactionInput::AccountInput(AccountInput {
                                            address: Address { id: address.into() },
                                            amount: value.0.into(),
                                        })
                                    }
                                    transaction::InputEnum::UtxoInput(utxo_pointer) => {
                                        TransactionInput::UtxoInput(UtxoInput {
                                            fragment: utxo_pointer.transaction_id.into(),
                                            offset: utxo_pointer.output_index,
                                        })
                                    }
                                },
                            )
                        },
                    )?;

                    Ok::<
                        Connection<
                            IndexCursor,
                            TransactionInput,
                            ConnectionFields<TransactionInputCount>,
                            EmptyFields,
                        >,
                        async_graphql::Error,
                    >(connection)
                })
                .await
                .unwrap()
            },
        )
        .await
    }

    pub async fn outputs(
        &self,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<String>,
        after: Option<String>,
    ) -> FieldResult<
        Connection<
            IndexCursor,
            TransactionOutput,
            ConnectionFields<TransactionOutputCount>,
            EmptyFields,
        >,
    > {
        let id = self.id.clone();
        let txn = Arc::clone(&self.txn);
        query(
            after,
            before,
            first,
            last,
            move |after, before, first, last| async move {
                tokio::task::spawn_blocking(move || {
                    let outputs = txn
                        .get_fragment_outputs(&id)
                        .map_err(|_| ApiError::InternalDbError)?;

                    let connection = get_page::<TransactionOutputCount, _, _, _, _, _, _, _>(
                        outputs,
                        EdgeOrder::Increasing,
                        first,
                        last,
                        before,
                        after,
                        |h, output| {
                            Edge::new(
                                IndexCursor::from(h as u32),
                                TransactionOutput {
                                    amount: output.value.get().into(),
                                    address: Address {
                                        id: output.address.clone(),
                                    },
                                },
                            )
                        },
                    )?;

                    Ok::<
                        Connection<
                            IndexCursor,
                            TransactionOutput,
                            ConnectionFields<TransactionOutputCount>,
                            EmptyFields,
                        >,
                        async_graphql::Error,
                    >(connection)
                })
                .await
                .unwrap()
            },
        )
        .await
    }

    pub async fn certificate(&self) -> FieldResult<Option<certificates::Certificate>> {
        let id = self.id.clone();
        let txn = Arc::clone(&self.txn);

        tokio::task::spawn_blocking(move || {
            let certificate = txn
                .get_fragment_certificate(&id)
                .map_err(|_| ApiError::InternalDbError)?;

            Ok(certificate.map(|cert| match cert.tag {
                CertificateTag::VotePlan => {
                    certificates::Certificate::VotePlan(VotePlanCertificate {
                        txn: Arc::clone(&txn),
                        meta: Mutex::new(None),
                        data: cert.clone().into_vote_plan().unwrap(),
                    })
                }
                CertificateTag::PublicVoteCast => {
                    certificates::Certificate::PublicVoteCast(PublicVoteCastCertificate {
                        data: cert.clone().into_public_vote_cast().unwrap(),
                    })
                }
                CertificateTag::PrivateVoteCast => {
                    certificates::Certificate::PrivateVoteCast(PrivateVoteCastCertificate {
                        data: cert.clone().into_private_vote_cast().unwrap(),
                    })
                }
            }))
        })
        .await
        .unwrap()
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

impl Address {
    fn from_bech32(bech32: &str) -> FieldResult<Address> {
        let addr = chain_addr::AddressReadable::from_string_anyprefix(bech32)
            .map_err(|_| ApiError::InvalidAddress(bech32.to_string()))?
            .to_address();

        Ok(Address { id: addr.into() })
    }
}

#[Object]
impl Address {
    /// The base32 representation of an address
    async fn id(&self, context: &Context<'_>) -> String {
        chain_addr::AddressReadable::from_address(
            &extract_context(context).settings.address_bech32_prefix,
            &self.id.clone().try_into().unwrap(),
        )
        .to_string()
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
    async fn block(&self, context: &Context<'_>, id: String) -> FieldResult<Block> {
        let txn = Arc::new(
            extract_context(&context)
                .db
                .get_txn()
                .await
                .map_err(|_| ApiError::InternalDbError)?,
        );

        Block::from_string_hash(id, txn).await
    }

    async fn blocks_by_chain_length(
        &self,
        context: &Context<'_>,
        length: ChainLength,
    ) -> FieldResult<Vec<Block>> {
        let txn = Arc::new(
            extract_context(&context)
                .db
                .get_txn()
                .await
                .map_err(|_| ApiError::InternalDbError)?,
        );

        let mut result = vec![];
        let chain_length = db::chain_storable::ChainLength::new(u32::from(length.0));

        let blocks = txn
            .get_blocks_by_chain_length(&chain_length)
            .map_err(|_| ApiError::InternalDbError)?;

        for block in blocks {
            result.push(Block::from_valid_hash(block?.clone(), Arc::clone(&txn)));
        }

        Ok(result)
    }

    async fn transaction(&self, context: &Context<'_>, id: String) -> FieldResult<Transaction> {
        let db = &extract_context(context).db;

        let id = chain_impl_mockchain::fragment::FragmentId::from_str(&id)?;

        let txn = db.get_txn().await.map_err(|_| ApiError::InternalDbError)?;

        tokio::task::spawn_blocking(move || {
            let id = db::chain_storable::FragmentId::from(id);
            let block_hashes = txn.transaction_blocks(&id)?;
            if block_hashes.is_empty() {
                Err(ApiError::NotFound(format!("transaction: {}", &id,)).into())
            } else {
                Ok(Transaction {
                    id,
                    block_hashes,
                    txn: Arc::new(txn),
                })
            }
        })
        .await?
    }

    /// get all current branch heads, sorted (descending) by their length
    pub async fn branches(&self, context: &Context<'_>) -> FieldResult<Vec<Branch>> {
        let db = &extract_context(context).db;

        let txn = Arc::new(db.get_txn().await.map_err(|_| ApiError::InternalDbError)?);

        tokio::task::spawn_blocking(move || {
            let branches = txn
                .get_branches()
                .map_err(|_| ApiError::InternalDbError)?
                .map(|branch| {
                    branch.map(|id| Branch {
                        id: id.clone(),
                        txn: Arc::clone(&txn),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;

            Ok(branches)
        })
        .await?
    }

    /// get the state that the ledger currently considers as the main branch
    async fn tip(&self, context: &Context<'_>) -> FieldResult<Branch> {
        let db = &extract_context(context).db;
        let txn = Arc::new(db.get_txn().await.map_err(|_| ApiError::InternalDbError)?);

        tokio::task::spawn_blocking(move || {
            let branch = txn
                .get_tip()
                .map_err(|_| ApiError::InternalDbError)
                .map(|id| Branch {
                    id,
                    txn: Arc::clone(&txn),
                })?;

            Ok(branch)
        })
        .await?
    }

    pub async fn branch(&self, _context: &Context<'_>, _id: String) -> FieldResult<Branch> {
        todo!()
    }

    pub async fn address(&self, _context: &Context<'_>, bech32: String) -> FieldResult<Address> {
        Address::from_bech32(&bech32)
    }
}

pub struct Subscription;

#[Subscription]
impl Subscription {
    async fn tip(&self, context: &Context<'_>) -> impl futures::Stream<Item = Branch> + '_ {
        use futures::StreamExt;
        let context = extract_context(context);

        let db = context.db.clone();

        tokio_stream::wrappers::BroadcastStream::new(context.tip_stream.subscribe())
            // missing a tip update doesn't seem that important, so I think it's
            // fine to ignore the error
            .filter_map(move |tip| {
                let db = db.clone();

                async move {
                    let txn = Arc::new(db.get_txn().await.unwrap());
                    tip.ok().map(|id| Branch { id: id.into(), txn })
                }
            })
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

pub enum EdgeOrder {
    Decreasing,
    Increasing,
}

/// tie the db::pagination abstraction with the graphql boundary computation.
//  TODO: the types in this are really complex, there is probably a better strategy,
//  although by the nature of this function it needs to be fairly generic.
//  currently the types are complex because it has both the type bounds of db::pagination and the
//  ones from graphql::pagination, and some extra ones to do conversion between them
fn get_page<'a, N, K, C, I, Item, F, M, Output>(
    mut edges: db::pagination::SanakirjaCursorIter<'a, K, I, C, M>,
    edge_order: EdgeOrder,
    first: Option<usize>,
    last: Option<usize>,
    before: Option<IndexCursor>,
    after: Option<IndexCursor>,
    map_item: F,
    // TODO: extract this to an enum, or better yet, to two different functions
) -> Result<Connection<IndexCursor, Item, ConnectionFields<N>>, FieldError>
where
    C: Clone + From<N> + sanakirja::Storable + db::pagination::PaginationCursor,
    K: sanakirja::Storable + PartialEq,
    N: From<C> + Clone + TryFrom<IndexCursor> + TryFrom<u64> + async_graphql::OutputType,
    I: sanakirja::Storable,
    u64: std::convert::From<N>,
    F: Fn(C, &Output) -> Edge<IndexCursor, Item, EmptyFields>,
    M: db::pagination::MapEntry<'a, K, I, C, Output = &'a Output>,
    Output: 'a,
{
    let boundaries = edges
        .first_cursor()
        .map(|first| {
            PaginationInterval::Inclusive(InclusivePaginationInterval {
                lower_bound: N::from(*first),
                upper_bound: N::from(*edges.last_cursor().unwrap()),
            })
        })
        .unwrap_or(PaginationInterval::Empty);

    let pagination_arguments = ValidatedPaginationArguments {
        first,
        last,
        before: before
            .map(TryInto::try_into)
            .transpose()
            .map_err(|_| ApiError::InvalidCursor("before".to_string()))?,
        after: after
            .map(TryInto::try_into)
            .transpose()
            .map_err(|_| ApiError::InvalidCursor("after".to_string()))?,
    };

    let (range, page_meta) = compute_interval(boundaries, pagination_arguments)?;
    let mut connection = Connection::with_additional_fields(
        page_meta.has_previous_page,
        page_meta.has_next_page,
        ConnectionFields {
            total_count: page_meta.total_count.try_into().map_err(|_| {
                ApiError::InternalError(
                    "pagination has more items than cursor type can hold".to_string(),
                )
            })?,
        },
    );

    match range {
        PaginationInterval::Empty => (),
        PaginationInterval::Inclusive(range) => {
            let a = C::from(range.lower_bound);
            let b = C::from(range.upper_bound);

            if matches!(edge_order, EdgeOrder::Decreasing) {
                edges.seek(b).map_err(|_| ApiError::InternalDbError)?;

                for edge in edges.rev() {
                    let (cursor, edge) = edge?;

                    if cursor >= a {
                        connection.append([map_item(cursor, edge)])
                    } else {
                        break;
                    }
                }
            } else {
                edges.seek(a).map_err(|_| ApiError::InternalDbError)?;

                for edge in edges {
                    let (cursor, edge) = edge?;

                    if cursor <= b {
                        connection.append([map_item(cursor, edge)])
                    } else {
                        break;
                    }
                }
            }
        }
    };

    Ok(connection)
}

use super::error::ErrorKind;
use super::scalars::{BlockCount, IndexCursor, TransactionCount};
use super::{Block, Context, Transaction};
use crate::blockcfg::HeaderHash;
use juniper::FieldResult;
use std::convert::TryFrom;

#[juniper::object(
    Context = Context
)]
impl PageInfo {
    pub fn has_next_page(&self) -> bool {
        self.has_next_page
    }

    pub fn has_previous_page(&self) -> bool {
        self.has_previous_page
    }

    pub fn start_cursor(&self) -> &Option<IndexCursor> {
        &self.start_cursor
    }

    pub fn end_cursor(&self) -> &Option<IndexCursor> {
        &self.end_cursor
    }
}

#[juniper::object(
    Context = Context
)]
impl BlockEdge {
    pub fn node(&self) -> &Block {
        &self.node
    }

    /// A cursor for use in pagination
    pub fn cursor(&self) -> &IndexCursor {
        &self.cursor
    }
}

#[juniper::object(
    Context = Context
)]
impl TransactionEdge {
    pub fn node(&self) -> &Transaction {
        &self.node
    }

    /// A cursor for use in pagination
    pub fn cursor(&self) -> &IndexCursor {
        &self.cursor
    }
}

#[juniper::object(
    Context = Context,
    name = "BlockConnection"
)]
impl BlockConnection {
    pub fn page_info(&self) -> &PageInfo {
        &self.page_info
    }

    pub fn edges(&self) -> &Vec<BlockEdge> {
        &self.edges
    }

    /// A count of the total number of objects in this connection, ignoring pagination.
    pub fn total_count(&self) -> &BlockCount {
        &self.total_count
    }
}

#[juniper::object(
    Context = Context,
    name = "TransactionConnection"
)]
impl TransactionConnection {
    pub fn page_info(&self) -> &PageInfo {
        &self.page_info
    }

    pub fn edges(&self) -> &Vec<TransactionEdge> {
        &self.edges
    }

    /// A count of the total number of objects in this connection, ignoring pagination.
    pub fn total_count(&self) -> &TransactionCount {
        &self.total_count
    }
}

pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub start_cursor: Option<IndexCursor>,
    pub end_cursor: Option<IndexCursor>,
}

pub struct Connection<E, C> {
    page_info: PageInfo,
    edges: Vec<E>,
    total_count: C,
}

pub struct TransactionEdge {
    node: Transaction,
    cursor: IndexCursor,
}

pub struct BlockEdge {
    pub node: Block,
    pub cursor: IndexCursor,
}

pub trait Edge {
    type Node;
    fn new(node: Self::Node, cursor: IndexCursor) -> Self;

    fn cursor<'a>(&'a self) -> &'a IndexCursor;
}

pub struct ValidatedPaginationArguments<I> {
    first: Option<u32>,
    last: Option<u32>,
    before: Option<I>,
    after: Option<I>,
}

pub struct PaginationArguments<I> {
    pub first: Option<i32>,
    pub last: Option<i32>,
    pub before: Option<I>,
    pub after: Option<I>,
}

impl<E, C> Connection<E, C>
where
    E: Edge,
    C: From<u64>,
    E::Node: Clone,
{
    pub fn new<I>(
        lower_bound: I,
        upper_bound: I,
        pagination_arguments: ValidatedPaginationArguments<I>,
        get_node_range: impl Fn(I, I) -> Vec<(E::Node, I)>,
    ) -> FieldResult<Connection<E, C>>
    where
        I: TryFrom<u64>,
        u64: From<I>,
        I: Clone,
        IndexCursor: From<I>,
    {
        let lower_bound: u64 = lower_bound.into();
        let upper_bound: u64 = upper_bound.into();
        let pagination_arguments = pagination_arguments.cursors_into::<u64>();

        let [from, to] = compute_range_boundaries(lower_bound, upper_bound, pagination_arguments)?;

        let has_next_page = to < upper_bound;
        let has_previous_page = from > lower_bound;

        let index_from = I::try_from(from)
            .map_err(|_| "page range is out of boundaries")
            .unwrap();
        let index_to = I::try_from(to)
            .map_err(|_| "page range is out of boundaries")
            .unwrap();

        let edges: Vec<_> = get_node_range(index_from, index_to)
            .iter()
            .map(|(hash, node_pagination_identifier)| {
                E::new((*hash).clone(), node_pagination_identifier.clone().into())
            })
            .collect();

        let start_cursor = edges.first().map(|e| e.cursor().clone());
        let end_cursor = edges
            .last()
            .map(|e| e.cursor().clone())
            .or(start_cursor.clone());

        Ok(Connection {
            edges,
            page_info: PageInfo {
                has_next_page,
                has_previous_page,
                start_cursor,
                end_cursor,
            },
            total_count: (upper_bound
                .checked_sub(lower_bound)
                .expect("upper_bound to be >= than lower_bound"))
            .into(),
        })
    }
}

pub type BlockConnection = Connection<BlockEdge, BlockCount>;
pub type TransactionConnection = Connection<TransactionEdge, TransactionCount>;

impl Edge for TransactionEdge {
    type Node = HeaderHash;
    fn new(node: Self::Node, cursor: IndexCursor) -> TransactionEdge {
        TransactionEdge {
            node: Transaction::from_valid_id(node),
            cursor,
        }
    }

    fn cursor(&self) -> &IndexCursor {
        &self.cursor
    }
}

impl Edge for BlockEdge {
    type Node = HeaderHash;
    fn new(node: Self::Node, cursor: IndexCursor) -> Self {
        BlockEdge {
            node: Block::from_valid_hash(node),
            cursor,
        }
    }

    fn cursor<'a>(&'a self) -> &'a IndexCursor {
        &self.cursor
    }
}

fn compute_range_boundaries(
    lower_bound: u64,
    upper_bound: u64,
    pagination_arguments: ValidatedPaginationArguments<u64>,
) -> FieldResult<[u64; 2]>
where
{
    use std::cmp::{max, min};

    // Compute the required range of blocks in two variables: [from, to]
    // Both ends are inclusive
    let mut from: u64 = match pagination_arguments.after {
        Some(cursor) => max(cursor + 1, lower_bound),
        // If `after` is not set, start from the beginning
        None => lower_bound,
    }
    .into();

    let mut to: u64 = match pagination_arguments.before {
        Some(cursor) => min(cursor - 1, upper_bound),
        // If `before` is not set, start from the beginning
        None => upper_bound,
    }
    .into();

    // Move `to` enough values to make the result have `first` blocks
    if let Some(first) = pagination_arguments.first {
        to = min(
            from.checked_add(u64::try_from(first).unwrap())
                .unwrap_or(to),
            to,
        );
    }

    // Move `from` enough values to make the result have `last` blocks
    if let Some(last) = pagination_arguments.last {
        from = max(
            to.checked_sub(u64::try_from(last).unwrap())
                .unwrap_or(from),
            from,
        );
    }

    Ok([from, to])
}

impl<I> PaginationArguments<I> {
    pub fn validate(self) -> FieldResult<ValidatedPaginationArguments<I>> {
        let first = self
            .first
            .map(|signed| -> FieldResult<u32> {
                if signed < 0 {
                    return Err(ErrorKind::ArgumentError(
                        "first argument should be positive".to_owned(),
                    )
                    .into());
                } else {
                    Ok(u32::try_from(signed).unwrap())
                }
            })
            .transpose()?;

        let last = self
            .last
            .map(|signed| -> FieldResult<u32> {
                if signed < 0 {
                    return Err(ErrorKind::ArgumentError(
                        "last argument should be positive".to_owned(),
                    )
                    .into());
                } else {
                    Ok(u32::try_from(signed).unwrap())
                }
            })
            .transpose()?;

        let before = self.before;
        let after = self.after;

        Ok(ValidatedPaginationArguments {
            first,
            after,
            last,
            before,
        })
    }
}

impl<I> ValidatedPaginationArguments<I> {
    fn cursors_into<T>(self) -> ValidatedPaginationArguments<T>
    where
        T: From<I>,
    {
        ValidatedPaginationArguments {
            after: self.after.map(T::from),
            before: self.before.map(T::from),
            first: self.first,
            last: self.last,
        }
    }
}

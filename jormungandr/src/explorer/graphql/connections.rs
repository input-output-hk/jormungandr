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

impl<E, C> Connection<E, C>
where
    E: Edge,
    C: From<u32>,
    E::Node: Clone,
{
    pub fn new<I>(
        lower_bound: u32,
        upper_bound: u32,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<IndexCursor>,
        after: Option<IndexCursor>,
        get_node_range: impl Fn(I, I) -> Vec<(E::Node, I)>,
    ) -> FieldResult<Connection<E, C>>
    where
        u32: From<I>,
        I: From<u32> + Clone,
    {
        let before: Option<u32> = before.map(|i: IndexCursor| -> u32 { i.into() });
        let after: Option<u32> = after.map(|i: IndexCursor| -> u32 { i.into() });

        let (from, to) =
            compute_range_boundaries(lower_bound, upper_bound, last, before, first, after)?;

        let has_next_page = to < upper_bound;
        let has_previous_page = from > lower_bound;
        let edges: Vec<_> = get_node_range(I::from(from), I::from(to))
            .iter()
            .map(|(hash, node_pagination_identifier)| {
                E::new(
                    (*hash).clone(),
                    IndexCursor::from(u32::from(node_pagination_identifier.clone())),
                )
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
    lower_bound: u32,
    upper_bound: u32,
    last: Option<i32>,
    before: Option<u32>,
    first: Option<i32>,
    after: Option<u32>,
) -> FieldResult<(u32, u32)> {
    use std::cmp::{max, min};
    // Compute the required range of blocks in two variables: [from, to]
    // Both ends are inclusive
    let mut from = match after {
        Some(cursor) => max(cursor + 1, lower_bound),
        // If `after` is not set, start from the beginning
        None => lower_bound,
    };

    let mut to = match before {
        Some(cursor) => min(cursor - 1, upper_bound),
        // If `before` is not set, start from the beginning
        None => upper_bound,
    };

    // Move `to` enough values to make the result have `first` blocks
    if let Some(first) = first {
        if first < 0 {
            return Err(
                ErrorKind::ArgumentError("first argument should be positive".to_owned()).into(),
            );
        } else {
            to = min(
                from.checked_add(u32::try_from(first).unwrap())
                    .map(|n| n - 1)
                    .unwrap_or(to),
                to,
            );
        }
    }

    // Move `from` enough values to make the result have `last` blocks
    if let Some(last) = last {
        if last < 0 {
            return Err(
                ErrorKind::ArgumentError("last argument should be positive".to_owned()).into(),
            );
        } else {
            from = max(
                to.checked_sub(u32::try_from(last).unwrap())
                    .map(|n| n + 1)
                    .unwrap_or(from),
                from,
            );
        }
    }

    Ok((from, to))
}

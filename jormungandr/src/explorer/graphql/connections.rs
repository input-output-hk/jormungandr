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
        bounds: PaginationInterval<I>,
        pagination_arguments: ValidatedPaginationArguments<I>,
        get_node_range: impl Fn(PaginationInterval<I>) -> Vec<(E::Node, I)>,
    ) -> FieldResult<Connection<E, C>>
    where
        I: TryFrom<u64>,
        u64: From<I>,
        I: Clone,
        IndexCursor: From<I>,
    {
        let pagination_arguments = pagination_arguments.cursors_into::<u64>();
        let bounds = bounds.bounds_into::<u64>();

        let (page_interval, has_next_page, has_previous_page, total_count) = match bounds {
            PaginationInterval::Empty => (PaginationInterval::Empty, false, false, 0.into()),
            PaginationInterval::Inclusive(total_elements) => {
                let InclusivePaginationInterval {
                    upper_bound,
                    lower_bound,
                } = total_elements;

                let page = compute_range_boundaries(total_elements, pagination_arguments)?;

                let has_next_page = page.upper_bound < upper_bound;
                let has_previous_page = page.lower_bound > lower_bound;

                let total_count = upper_bound
                    .checked_sub(lower_bound)
                    .expect("upper_bound should be >= than lower_bound")
                    .into();
                (
                    PaginationInterval::Inclusive(page),
                    has_next_page,
                    has_previous_page,
                    total_count,
                )
            }
        };

        let page_interval = page_interval
            .bounds_try_into::<I>()
            .map_err(|_| "computed page interval is outside pagination boundaries")
            .unwrap();

        let edges: Vec<_> = get_node_range(page_interval)
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
            total_count,
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
    total_elements: InclusivePaginationInterval<u64>,
    pagination_arguments: ValidatedPaginationArguments<u64>,
) -> FieldResult<InclusivePaginationInterval<u64>>
where
{
    use std::cmp::{max, min};

    let InclusivePaginationInterval {
        upper_bound,
        lower_bound,
    } = total_elements;

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
            from.checked_add(u64::from(first))
                .and_then(|n| n.checked_sub(1))
                .unwrap_or(to),
            to,
        );
    }

    // Move `from` enough values to make the result have `last` blocks
    if let Some(last) = pagination_arguments.last {
        from = max(
            to.checked_sub(u64::from(last))
                .and_then(|n| n.checked_add(1))
                .unwrap_or(from),
            from,
        );
    }

    Ok(InclusivePaginationInterval {
        lower_bound: from,
        upper_bound: to,
    })
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

pub enum PaginationInterval<I> {
    Empty,
    Inclusive(InclusivePaginationInterval<I>),
}

pub struct InclusivePaginationInterval<I> {
    pub lower_bound: I,
    pub upper_bound: I,
}

impl<I> PaginationInterval<I> {
    fn bounds_into<T>(self) -> PaginationInterval<T>
    where
        T: From<I>,
    {
        match self {
            Self::Empty => PaginationInterval::<T>::Empty,
            Self::Inclusive(interval) => {
                PaginationInterval::<T>::Inclusive(InclusivePaginationInterval::<T> {
                    lower_bound: T::from(interval.lower_bound),
                    upper_bound: T::from(interval.upper_bound),
                })
            }
        }
    }

    fn bounds_try_into<T>(self) -> Result<PaginationInterval<T>, <T as TryFrom<I>>::Error>
    where
        T: TryFrom<I>,
    {
        match self {
            Self::Empty => Ok(PaginationInterval::<T>::Empty),
            Self::Inclusive(interval) => Ok(PaginationInterval::<T>::Inclusive(
                InclusivePaginationInterval::<T> {
                    lower_bound: T::try_from(interval.lower_bound)?,
                    upper_bound: T::try_from(interval.upper_bound)?,
                },
            )),
        }
    }
}

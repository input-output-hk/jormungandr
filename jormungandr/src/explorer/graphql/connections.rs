use super::error::ErrorKind;
use super::scalars::BlockCount;
use super::{Block, Context};
use blockcfg::{self, HeaderHash};
use juniper::{FieldResult, ParseScalarResult, ParseScalarValue, Value};
use std::convert::TryFrom;

#[derive(Clone)]
pub struct BlockCursor(blockcfg::ChainLength);

juniper::graphql_scalar!(BlockCursor where Scalar = <S> {
    description: "Opaque cursor to use in block pagination, a client should not rely in its representation"

    // FIXME: Cursors are recommended to be opaque, but I'm not sure it is worth to
    // obfuscate its representation
    resolve(&self) -> Value {
        Value::scalar(self.0.to_string())
    }

    from_input_value(v: &InputValue) -> Option<BlockCursor> {
        v.as_scalar_value::<String>()
         .and_then(|s| s.parse::<u32>().ok())
         .map(|n| BlockCursor(blockcfg::ChainLength::from(n)))
    }

    from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        <String as ParseScalarValue<S>>::from_str(value)
    }
});

impl From<u32> for BlockCursor {
    fn from(number: u32) -> BlockCursor {
        BlockCursor(blockcfg::ChainLength::from(number))
    }
}

impl From<BlockCursor> for u32 {
    fn from(number: BlockCursor) -> u32 {
        number.0.into()
    }
}

impl From<blockcfg::ChainLength> for BlockCursor {
    fn from(length: blockcfg::ChainLength) -> BlockCursor {
        BlockCursor(length)
    }
}

impl From<BlockCursor> for blockcfg::ChainLength {
    fn from(c: BlockCursor) -> blockcfg::ChainLength {
        c.0.into()
    }
}

pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub start_cursor: BlockCursor,
    pub end_cursor: BlockCursor,
}

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

    pub fn start_cursor(&self) -> &BlockCursor {
        &self.start_cursor
    }

    pub fn end_cursor(&self) -> &BlockCursor {
        &self.end_cursor
    }
}

pub struct BlockEdge {
    pub node: Block,
    pub cursor: BlockCursor,
}

#[juniper::object(
    Context = Context
)]
impl BlockEdge {
    pub fn node(&self) -> &Block {
        &self.node
    }

    /// A cursor for use in pagination
    pub fn cursor(&self) -> &BlockCursor {
        &self.cursor
    }
}

pub struct BlockConnection {
    pub page_info: PageInfo,
    pub edges: Vec<BlockEdge>,
    pub total_count: BlockCount,
}

#[juniper::object(
    Context = Context
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

impl BlockConnection {
    // The lower and upper bound are used to define all the blocks this connection will show
    // In particular, they are used to paginate Epoch blocks from first block in epoch to
    // last.
    pub fn new<I>(
        lower_bound: u32,
        upper_bound: u32,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<u32>,
        after: Option<u32>,
        get_block_range: impl Fn(I, I) -> Vec<(HeaderHash, I)>,
    ) -> FieldResult<BlockConnection>
    where
        u32: From<I>,
        I: From<u32> + Clone,
    {
        use std::cmp::{max, min};

        // Compute the required range of blocks in two variables: [from, to]
        // Both ends are inclusive
        let mut from = match after {
            Some(cursor) => cursor + 1,
            // If `after` is not set, start from the beginning
            None => lower_bound,
        };

        let mut to = match before {
            Some(cursor) => cursor - 1,
            // If `before` is not set, start from the beginning
            None => upper_bound,
        };

        // Move `to` enough values to make the result have `first` blocks
        if let Some(first) = first {
            if first < 0 {
                return Err(ErrorKind::ArgumentError(
                    "first argument should be positive".to_owned(),
                )
                .into());
            } else {
                to = min(
                    from.checked_add(u32::try_from(first).unwrap())
                        .map(|n| n - 1)
                        .or(Some(to))
                        .unwrap(),
                    to,
                );
            }
        }

        // Move `from` enough values to make the result have `last` blocks
        if let Some(last) = last {
            if last < 0 {
                return Err(ErrorKind::ArgumentError(
                    "last argument should be positive".to_owned(),
                )
                .into());
            } else {
                from = max(
                    to.checked_sub(u32::try_from(last).unwrap())
                        .map(|n| n + 1)
                        .or(Some(from))
                        .unwrap(),
                    from,
                );
            }
        }

        let has_next_page = to < upper_bound;
        let has_previous_page = from > lower_bound;
        let edges: Vec<_> = get_block_range(from.into(), (to + 1).into())
            .iter()
            .map(|(hash, block_pagination_identifier)| BlockEdge {
                node: Block::from_valid_hash(*hash),
                cursor: BlockCursor::from(u32::from(block_pagination_identifier.clone())),
            })
            .collect();

        let start_cursor = edges.first().expect("to be at least 1 edge").cursor.clone();
        let end_cursor = edges
            .last()
            .map(|e| e.cursor.clone())
            .unwrap_or(start_cursor.clone());

        Ok(BlockConnection {
            edges,
            page_info: PageInfo {
                has_next_page,
                has_previous_page,
                start_cursor,
                end_cursor,
            },
            total_count: (upper_bound - lower_bound).into(),
        })
    }
}

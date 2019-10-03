use super::error::ErrorKind;
use super::scalars::BlockCount;
use super::{Block, Context, ExplorerDB};
use blockcfg;
use futures::Future;
use juniper::{FieldResult, ParseScalarResult, ParseScalarValue, Value};
use std::convert::TryFrom;

pub struct BlockCursor(blockcfg::ChainLength);

juniper::graphql_scalar!(BlockCursor where Scalar = <S> {
    description: "Opaque cursor to use in block pagination"

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

    pub fn total_count(&self) -> &BlockCount {
        &self.total_count
    }
}

impl BlockConnection {
    pub fn new(
        lower_bound: BlockCursor,
        upper_bound: BlockCursor,
        first: Option<i32>,
        last: Option<i32>,
        before: Option<BlockCursor>,
        after: Option<BlockCursor>,
        db: &ExplorerDB,
    ) -> FieldResult<BlockConnection> {
        use std::cmp::{max, min};

        let lower_bound = u32::from(lower_bound);
        let upper_bound = u32::from(upper_bound);

        // Compute the required range of blocks in two variables: [from, to]
        // Both ends are inclusive
        let mut from = match after {
            Some(cursor) => u32::from(cursor) + 1,
            // If `after` is not set, start from the beginning
            None => lower_bound,
        };

        let mut to = match before {
            Some(cursor) => u32::from(cursor) - 1,
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
                        .or(Some(to))
                        .unwrap()
                        - 1,
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
                    u32::from(to)
                        .checked_sub(u32::try_from(last).unwrap())
                        .or(Some(from))
                        .unwrap()
                        + 1,
                    from,
                );
            }
        }

        let has_next_page = to < upper_bound;
        let has_previous_page = from > lower_bound;
        let edges = db
            .get_block_hash_range(from.into(), (to + 1).into())
            .wait()?
            .iter()
            .map(|(hash, chain_length)| BlockEdge {
                node: Block::from_valid_hash(*hash),
                cursor: (*chain_length).into(),
            })
            .collect();

        Ok(BlockConnection {
            edges,
            page_info: PageInfo {
                has_next_page,
                has_previous_page,
                start_cursor: lower_bound.into(),
                end_cursor: upper_bound.into(),
            },
            total_count: (upper_bound - lower_bound).into(),
        })
    }
}

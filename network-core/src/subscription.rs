use chain_core::property::{Block, HasHeader};

use std::fmt::{self, Debug};

pub enum BlockEvent<B>
where
    B: Block + HasHeader,
{
    Announce(B::Header),
    Solicit(Vec<B::Id>),
    Missing { from: Vec<B::Id>, to: B::Id },
}

impl<B> Debug for BlockEvent<B>
where
    B: Block + HasHeader,
    <B as HasHeader>::Header: Debug,
    <B as Block>::Id: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BlockEvent::Announce(header) => f.debug_tuple("Announce").field(header).finish(),
            BlockEvent::Solicit(ids) => f.debug_tuple("Solicit").field(ids).finish(),
            BlockEvent::Missing { from, to } => f
                .debug_struct("Missing")
                .field("from", from)
                .field("to", to)
                .finish(),
        }
    }
}

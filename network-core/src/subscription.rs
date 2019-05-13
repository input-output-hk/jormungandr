use chain_core::property::{Block, HasHeader};

use std::fmt::{self, Debug};

pub enum BlockEvent<B>
where
    B: Block + HasHeader,
{
    Announce(<B as HasHeader>::Header),
    Solicit(Vec<<B as Block>::Id>),
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
        }
    }
}

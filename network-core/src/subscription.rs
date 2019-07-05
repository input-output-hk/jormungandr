use chain_core::property::{Block, HasHeader};

use std::fmt::{self, Debug};

pub enum BlockEvent<B>
where
    B: Block + HasHeader,
{
    Announce(B::Header),
    Solicit(Vec<B::Id>),
    Missing(ChainPullRequest<B::Id>),
}

/// A request to send headers in the block chain sequence.
#[derive(Debug)]
pub struct ChainPullRequest<Id> {
    /// A list of starting points known by the requester.
    /// The sender should pick the latest one.
    pub from: Vec<Id>,
    /// The identifier of the last block to send the header for.
    pub to: Id,
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
            BlockEvent::Missing(req) => f.debug_tuple("Missing").field(req).finish(),
        }
    }
}

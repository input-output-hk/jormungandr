use blockcfg::{Block, Header, BlockHash};
use std::{sync::{Arc}, fmt, ops::{Deref}};

/// Simple RAII for the Reply lambda wrapping
#[derive(Clone)]
pub struct Reply<A>(Arc<Fn(A) + 'static + Send + Sync>);
impl<A, F> From<F> for Reply<A>
where F: Fn(A) + 'static + Send + Sync,
{
    fn from(f: F) -> Self { Reply(Arc::new(f)) }
}
impl<A> fmt::Debug for Reply<A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Reply<T>")
    }
}
impl<A> Deref for Reply<A> {
    type Target = Fn(A) + 'static + Send + Sync;
    fn deref(&self) -> &Self::Target { self.0.deref() }
}

// TODO

pub type TransactionMsg = u32;

/// Client messages, mainly requests from connected peers to our node.
/// Fetching the block headers, the block, the tip
#[derive(Debug, Clone)]
pub enum ClientMsg {
    GetBlockTip(Reply<Header>),
    GetBlockHeaders(Vec<BlockHash>, BlockHash, Reply<Vec<Header>>),
    GetBlocks(BlockHash, BlockHash, Reply<Vec<Block>>),
}

/// General Block Message for the block task
#[derive(Debug, Clone)]
pub enum BlockMsg {
    /// A untrusted Block has been received from the network task
    NetworkBlock(Block),
    /// A trusted Block has been received from the leadership task
    LeadershipBlock(Block),
}

#[cfg(test)]
mod test {
    use super::{*};

    #[test]
    fn reply_test() {
        let reply : Reply<u32> = Reply::from(|v| {
            println!("value: {}", v)
        });

        (*reply)(42u32);
    }
}

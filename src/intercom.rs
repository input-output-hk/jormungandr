use blockcfg::Block;

// TODO

pub type TransactionMsg = u32;
pub type ClientMsg = u32;

/// General Block Message for the block task
pub enum BlockMsg {
    /// A untrusted Block has been received from the network task
    NetworkBlock(Block),
    /// A trusted Block has been received from the leadership task
    LeadershipBlock(Block),
}

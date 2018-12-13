use crate::blockcfg::chain::Block;
use crate::blockcfg::chain::cardano;

/// trait to manage dynamic updates on the blockchain protocols
///
/// fee algorithm, number of transactions per block, etc...
pub trait Update {
    type Block: Block;

    /// get the number of transaction per block
    fn number_transactions_per_block(&self) -> usize;

    fn get_tip(&self) -> <Self::Block as Block>::Hash;
}

impl Update for ::cardano::block::verify_chain::ChainState {
    type Block = cardano::Block;

    fn number_transactions_per_block(&self) -> usize {
        self.nr_transactions as usize
    }

    fn get_tip(&self) -> <Self::Block as Block>::Hash {
        self.last_block.clone()
    }
}

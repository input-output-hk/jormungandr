use cardano;

use blockcfg::chain;
use blockcfg::ledger;

pub type GenesisData = cardano::config::GenesisData;
pub type TransactionId = cardano::tx::TxId;
pub type Transaction = cardano::tx::TxAux;
pub type BlockHash = cardano::block::HeaderHash;
pub type Block = cardano::block::Block;
pub type Header = cardano::block::BlockHeader;

impl chain::Block for Block {
    type Hash = BlockHash;
    type Id = cardano::block::BlockDate;

    fn parent_hash(&self) -> &Self::Hash {
        match self {
            cardano::block::Block::BoundaryBlock(ref bb) => {
                &bb.header.previous_header
            }
            cardano::block::Block::MainBlock(ref mb) => {
                &mb.header.previous_header
            }
        }
    }
    fn slot_id(&self) -> Self::Id {
        self.get_header().get_slotid()
    }
}
impl ledger::HasTransaction for Block {
    type Transaction = Transaction;

    fn transactions<'a>(&'a self) -> std::slice::Iter<'a, Self::Transaction>
    {
        match self {
            cardano::block::Block::BoundaryBlock(ref _bb) => {
                [].iter()
            }
            cardano::block::Block::MainBlock(ref mb) => {
                mb.body.tx.iter()
            }
        }
    }
}
impl ledger::Transaction for Transaction {
    type Input  = cardano::tx::TxoPointer;
    type Output = cardano::tx::TxOut;
    type Id = TransactionId;
    fn id(&self) -> Self::Id {
        self.tx.id()
    }
}
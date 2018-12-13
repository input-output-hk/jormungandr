use std::sync::Arc;
use super::selection::{self, IsLeading, Selection};

use super::super::{
    clock, BlockchainR, utils::task::{TaskMessageBox}, intercom::{BlockMsg}, secure::NodeSecret,
};
use crate::blockcfg::{BlockConfig, chain, update::Update};
use crate::transaction::{TPoolR};

use cardano::block::{EpochSlotId, BlockDate};

pub fn leadership_task<B>(
    secret: NodeSecret,
    selection: Arc<Selection>,
    transaction_pool: TPoolR<B>,
    blockchain: BlockchainR<B>,
    clock: clock::Clock,
    block_task: TaskMessageBox<BlockMsg<B>>
)
  where B: BlockConfig
      , <B as BlockConfig>::TransactionId: Eq + std::hash::Hash
      , <B as BlockConfig>::Ledger: Update
      , <B as BlockConfig>::Block : chain::Block<Id = BlockDate>
{
    let my_pub = secret.public.clone();
    loop {
        let d = clock.wait_next_slot();
        let (epoch, idx, next_time) = clock.current_slot().unwrap();
        debug!("slept for {:?} epoch {} slot {} next_slot {:?}", d, epoch.0, idx, next_time);

        // TODO in the future "current stake" will be one of the parameter
        let leader = selection::test(&selection, idx as u64);

        if leader == IsLeading::Yes {
            // if we have the leadership to create a new block we can require the lock
            // on the blockchain as we are not expecting to be _blocked_ while creating
            // the block.
            let b = blockchain.read().unwrap();

            // collect up to `nr_transactions` from the transaction pool.
            //
            let transactions =
                transaction_pool.write().unwrap().collect(b.chain_state.number_transactions_per_block());

            let epochslot = EpochSlotId { epoch: epoch.0 as u64, slotid: idx as u16 };
            info!("leadership create tpool={} transactions ({})", transactions.len(), epochslot);

            let block = B::make_block(&secret, &my_pub, &b.chain_state, BlockDate::Normal(epochslot), transactions);

            block_task.send_to(
                BlockMsg::LeadershipBlock(
                    block
                )
            );
        }
    }
}

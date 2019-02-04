use crate::blockcfg::{BlockConfig, Settings};
use crate::transaction::TPoolR;
use crate::{clock, intercom::BlockMsg, utils::task::TaskMessageBox, BlockchainR};

use chain_core::property::{BlockDate, LeaderSelection};

pub fn leadership_task<B>(
    secret: <B as BlockConfig>::NodeSigningKey,
    transaction_pool: TPoolR<B>,
    blockchain: BlockchainR<B>,
    clock: clock::Clock,
    block_task: TaskMessageBox<BlockMsg<B>>,
) where
    B: BlockConfig,
    <B as BlockConfig>::TransactionId: Eq + std::hash::Hash,
    <B as BlockConfig>::Settings: Settings,
{
    loop {
        let d = clock.wait_next_slot();
        let (epoch, idx, next_time) = clock.current_slot().unwrap();

        debug!(
            "slept for {:?} epoch {} slot {} next_slot {:?}",
            d, epoch.0, idx, next_time
        );

        let date = <B::BlockDate as BlockDate>::from_epoch_slot_id(epoch.0 as u64, idx as u64);

        // if we have the leadership to create a new block we can require the lock
        // on the blockchain as we are not expecting to be _blocked_ while creating
        // the block.
        let b = blockchain.read().unwrap();

        let is_leader = b.leadership.is_leader_at(date).unwrap();

        if is_leader {
            // collect up to `nr_transactions` from the transaction pool.
            //
            let transactions = transaction_pool
                .write()
                .unwrap()
                .collect(b.settings.max_number_of_transactions_per_block());

            info!(
                "leadership create tpool={} transactions ({}.{})",
                transactions.len(),
                epoch.0,
                idx
            );

            let block = B::make_block(&secret, &b.settings, &b.ledger, date, transactions);

            block_task.send_to(BlockMsg::LeadershipBlock(block));
        }
    }
}

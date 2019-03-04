use crate::blockcfg::{BlockConfig, Settings};
use crate::transaction::TPoolR;
use crate::{clock, intercom::BlockMsg, utils::task::TaskMessageBox, BlockchainR};

use chain_core::property::{BlockDate, LeaderSelection};

pub fn leadership_task<B>(
    leader_id: <<B as BlockConfig>::Leader as LeaderSelection>::LeaderId,
    secret: <B as BlockConfig>::NodeSigningKey,
    transaction_pool: TPoolR<B>,
    blockchain: BlockchainR<B>,
    clock: clock::Clock,
    block_task: TaskMessageBox<BlockMsg<B>>,
) where
    B: BlockConfig,
    <B as BlockConfig>::TransactionId: Eq + std::hash::Hash,
    <B as BlockConfig>::Settings: Settings,
    // FIXME: LeaderId should always require PartialEq.
    <<B as BlockConfig>::Leader as LeaderSelection>::LeaderId: PartialEq,
{
    loop {
        let d = clock.wait_next_slot();
        let (epoch, idx, next_time) = clock.current_slot().unwrap();

        debug!(
            "slept for {:?} epoch {} slot {} next_slot {:?}",
            d, epoch.0, idx, next_time
        );

        let date = <B::BlockDate as BlockDate>::from_epoch_slot_id(epoch.0, idx);

        // if we have the leadership to create a new block we can require the lock
        // on the blockchain as we are not expecting to be _blocked_ while creating
        // the block.
        let b = blockchain.read().unwrap();

        let am_leader = b.state.leaders.get_leader_at(date.clone()).unwrap() == leader_id;

        if am_leader {
            // collect up to `nr_transactions` from the transaction pool.
            //
            let transactions = transaction_pool.write().unwrap().collect(
                b.state
                    .settings
                    .read()
                    .unwrap()
                    .max_number_of_transactions_per_block() as usize,
            );

            info!(
                "leadership create tpool={} transactions ({}.{})",
                transactions.len(),
                epoch.0,
                idx
            );

            let block = B::make_block(
                &secret,
                &b.state.settings.read().unwrap(),
                &b.state.ledger.read().unwrap(),
                date,
                transactions,
            );

            block_task.send_to(BlockMsg::LeadershipBlock(block));
        }
    }
}

use crate::{
    blockcfg::{BlockDate, Leader, LeaderId, Leadership},
    clock,
    intercom::BlockMsg,
    transaction::TPoolR,
    utils::task::TaskMessageBox,
    BlockchainR,
};
use chain_core::property::{Block as _, BlockDate as _, ChainLength, LeaderSelection};

pub fn leadership_task(
    leader_id: LeaderId,
    secret: Leader,
    transaction_pool: TPoolR,
    blockchain: BlockchainR,
    clock: clock::Clock,
    block_task: TaskMessageBox<BlockMsg>,
) {
    loop {
        let d = clock.wait_next_slot();
        let (epoch, idx, next_time) = clock.current_slot().unwrap();

        debug!(
            "slept for {:?} epoch {} slot {} next_slot {:?}",
            d, epoch.0, idx, next_time
        );

        let date = BlockDate::from_epoch_slot_id(epoch.0, idx);

        // if we have the leadership to create a new block we can require the lock
        // on the blockchain as we are not expecting to be _blocked_ while creating
        // the block.
        let b = blockchain.read().unwrap();
        let (last_block, _last_block_info) = b.get_block_tip().unwrap();
        let state = b.multiverse.get_from_root(&b.tip);
        let parameters = state.get_ledger_parameters();
        let leadership = Leadership::new(state);
        let parent_id = &*b.tip;
        let chain_length = last_block.chain_length().next();

        // let am_leader = leadership.get_leader_at(date.clone()).unwrap() == leader_id;
        let am_leader: bool = unimplemented!();

        if am_leader {
            // collect up to `nr_transactions` from the transaction pool.
            //
            let transactions = transaction_pool.write().unwrap().collect(250 /* TODO!! */);

            info!(
                "leadership create tpool={} transactions ({}.{})",
                transactions.len(),
                epoch.0,
                idx
            );

            let block = unimplemented!(); // make_block(&secret, date, chain_length, parent_id.clone(), transactions);

            block_task.send_to(BlockMsg::LeadershipBlock(block));
        }
    }
}

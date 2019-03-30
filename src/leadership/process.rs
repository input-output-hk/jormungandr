use crate::{
    blockcfg::{
        BlockBuilder, BlockDate, ChainLength, HeaderHash, Leader, LeaderOutput, Leadership,
    },
    clock,
    intercom::BlockMsg,
    transaction::TPoolR,
    utils::task::TaskMessageBox,
    BlockchainR,
};
use chain_core::property::{Block as _, BlockDate as _, ChainLength as _};

pub fn leadership_task(
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
        let b = blockchain.lock_read();
        let (last_block, _last_block_info) = b.get_block_tip().unwrap();
        let chain_length = last_block.chain_length().next();
        let state = b.multiverse.get_from_root(&b.tip);
        let leadership = Leadership::new(state);
        let parent_id = &*b.tip;

        // let am_leader = leadership.get_leader_at(date.clone()).unwrap() == leader_id;
        match leadership.is_leader(&secret, date).unwrap() {
            LeaderOutput::None => {}
            LeaderOutput::Bft(bft_secret_key) => {
                let block_builder =
                    prepare_block(&transaction_pool, date, chain_length, *parent_id);

                let block = block_builder.make_bft_block(bft_secret_key);

                block_task.send_to(BlockMsg::LeadershipBlock(block));
            }
            LeaderOutput::GenesisPraos => {
                // TODO
            }
        }
    }
}

fn prepare_block(
    transaction_pool: &TPoolR,
    date: BlockDate,
    chain_length: ChainLength,
    parent_id: HeaderHash,
) -> BlockBuilder {
    let mut bb = BlockBuilder::new();

    bb.date(date).parent(parent_id).chain_length(chain_length);
    let messages = transaction_pool.write().unwrap().collect(250 /* TODO!! */);
    bb.messages(messages);

    bb
}

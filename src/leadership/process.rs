use crate::{
    blockcfg::{Block, BlockBuilder, BlockDate, ChainLength, HeaderHash, Leader, LeaderOutput},
    clock,
    intercom::BlockMsg,
    transaction::TPoolR,
    utils::{async_msg::MessageBox, task::ThreadServiceInfo},
    BlockchainR,
};
use chain_core::property::{Block as _, BlockDate as _, ChainLength as _};
use slog::Logger;

pub fn leadership_task(
    service_info: ThreadServiceInfo,
    mut secret: Leader,
    transaction_pool: TPoolR,
    blockchain: BlockchainR,
    clock: clock::Clock,
    mut block_task: MessageBox<BlockMsg>,
) {
    loop {
        let d = clock.wait_next_slot().unwrap();
        let (epoch, idx, next_time) = clock.current_slot().unwrap();

        let date = BlockDate::from_epoch_slot_id(epoch.0, idx);

        let context_logger = Logger::root(
            service_info.logger().clone(),
            o!("date" => format!("{}.{}", date.epoch, date.slot_id)),
        );

        slog_debug!(
            context_logger,
            "slept for {}",
            humantime::format_duration(d),
        );
        slog_debug!(
            context_logger,
            "will sleep for {}",
            humantime::format_duration(next_time),
        );

        if let Some(block) = handle_event(
            &context_logger,
            &mut secret,
            &transaction_pool,
            &blockchain,
            date,
        ) {
            block_task.send(BlockMsg::LeadershipBlock(block));
        }
    }
}

fn handle_event(
    logger: &Logger,
    secret: &mut Leader,
    transaction_pool: &TPoolR,
    blockchain: &BlockchainR,
    date: BlockDate,
) -> Option<Block> {
    // if we have the leadership to create a new block we can require the lock
    // on the blockchain as we are not expecting to be _blocked_ while creating
    // the block.
    let b = blockchain.lock_read();
    let (last_block, _last_block_info) = b.get_block_tip().unwrap();
    let chain_length = last_block.chain_length().next();
    let state = b.get_ledger(&last_block.id()).unwrap();

    // get from the parameters the ConsensusVersion:
    let parameters = state.get_ledger_parameters();

    let leadership = // if parameters.consensus_version == ConsensusVersion::BFT {
            b.get_leadership(date.epoch).unwrap();
    // } else if parameters.consensus_version == ConsensusVersion::GenesisPraos {
    //    b.get_leadership(date.epoch.checked_sub(2).unwrap_or(date.epoch)).unwrap();
    // };
    let parent_id = &*b.tip;

    // let am_leader = leadership.get_leader_at(date.clone()).unwrap() == leader_id;
    match leadership.is_leader_for_date(&secret, date).unwrap() {
        LeaderOutput::None => None,
        LeaderOutput::Bft(_bft_public_key) => {
            if let Some(bft_secret_key) = &secret.bft_leader {
                slog_info!(logger, "Node elected for BFT");
                let block_builder =
                    prepare_block(&transaction_pool, date, chain_length, *parent_id);

                let block = block_builder.make_bft_block(&bft_secret_key.sig_key);

                assert!(leadership.verify(&block.header).success());
                Some(block)
            } else {
                None
            }
        }
        LeaderOutput::GenesisPraos(witness) => {
            if let Some(genesis_leader) = &mut secret.genesis_leader {
                slog_info!(logger, "Node elected for Genesis Praos");
                let block_builder =
                    prepare_block(&transaction_pool, date, chain_length, *parent_id);

                let block = block_builder.make_genesis_praos_block(
                    &genesis_leader.node_id,
                    &mut genesis_leader.sig_key,
                    witness,
                );

                assert!(leadership.verify(&block.header).success());
                Some(block)
            } else {
                None
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

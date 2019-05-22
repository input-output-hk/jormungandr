use crate::{
    blockcfg::{
        BlockBuilder, BlockDate, ChainLength, Epoch, HeaderHash, Leader, LeaderOutput,
        LedgerParameters, LedgerStaticParameters,
    },
    blockchain::Tip,
    intercom::BlockMsg,
    leadership::{LeaderSchedule, Leadership},
    transaction::TPoolR,
    utils::async_msg::MessageBox,
};
use chain_core::property::ChainLength as _;
use chain_time::timeframe::TimeFrame;
use slog::Logger;
use std::sync::{Arc, RwLock};
use tokio::{prelude::*, sync::watch};

custom_error! {pub HandleLeadershipError
    Schedule { source: tokio::timer::Error } = "Error in the leadership schedule",
}

custom_error! {pub TaskError
    LeadershipReceiver { extra: String } = "Cannot continue the leader task: {extra}",
    LeadershipHandle { source: HandleLeadershipError } = "Error while handling an epoch's leader schedule",
}

#[derive(Clone)]
pub struct TaskParameters {
    pub epoch: Epoch,
    pub ledger_static_parameters: LedgerStaticParameters,
    pub ledger_parameters: LedgerParameters,

    pub leadership: Arc<Leadership>,
    pub time_frame: TimeFrame,
}

pub struct Task {
    logger: Logger,
    leader: Arc<RwLock<Leader>>,
    blockchain_tip: Tip,
    epoch_receiver: watch::Receiver<Option<TaskParameters>>,
    transaction_pool: TPoolR,
    block_message: MessageBox<BlockMsg>,
}

impl Task {
    #[inline]
    pub fn new(
        logger: Logger,
        leader: Leader,
        blockchain_tip: Tip,
        transaction_pool: TPoolR,
        epoch_receiver: watch::Receiver<Option<TaskParameters>>,
        block_message: MessageBox<BlockMsg>,
    ) -> Self {
        let logger = Logger::root(
            logger,
            o!(
                "task" => "Leader Task",
                // TODO: add some general context information here (leader alias?)
            ),
        );

        Task {
            logger,
            leader: Arc::new(RwLock::new(leader)),
            blockchain_tip,
            transaction_pool,
            epoch_receiver,
            block_message,
        }
    }

    pub fn start(self) -> impl Future<Item = (), Error = ()> {
        let handle_logger = self.logger.clone();
        let crit_logger = self.logger;
        let leader = self.leader;
        let blockchain_tip = self.blockchain_tip;
        let transaction_pool = self.transaction_pool;
        let block_message = self.block_message;

        self.epoch_receiver
            .map_err(|error| TaskError::LeadershipReceiver {
                extra: format!("{}", error),
            })
            // filter_map so we don't have to do the pattern match on `Option::Nothing`.
            .filter_map(|task_parameters| task_parameters)
            .for_each(move |task_parameters| {
                handle_leadership(
                    block_message.clone(),
                    leader.clone(),
                    handle_logger.clone(),
                    blockchain_tip.clone(),
                    transaction_pool.clone(),
                    task_parameters,
                )
                .map_err(|error| {
                    TaskError::LeadershipHandle { source: error }
                })
            })
            .map_err(move |error| {
                crit!(crit_logger, "critical error in the Leader task" ; "reason" => error.to_string())
            })
    }
}

/// function that will run for the length of the Epoch associated
/// to the given leadership
///
fn handle_leadership(
    mut block_message: MessageBox<BlockMsg>,
    leader: Arc<RwLock<Leader>>,
    logger: Logger,
    blockchain_tip: Tip,
    transaction_pool: TPoolR,
    task_parameters: TaskParameters,
) -> impl Future<Item = (), Error = HandleLeadershipError> {
    let era = task_parameters.leadership.era().clone();
    let time_frame = task_parameters.time_frame.clone();

    let current_slot = time_frame.slot_at(&std::time::SystemTime::now()).expect(
        "assume we cannot only get one valid timeline and that the slot duration does not change",
    );
    let epoch_position = era
        .from_slot_to_era(current_slot)
        .expect("assume the current time is already in the era");

    // TODO: need to handle:
    //
    // * if too late for this leadership, log it and return
    assert!(epoch_position.epoch.0 <= task_parameters.epoch);

    let schedule = LeaderSchedule::new(logger.clone(), &leader.read().unwrap(), &task_parameters);

    schedule
        .map_err(|err| HandleLeadershipError::Schedule { source: err })
        .for_each(move |scheduled_event| {
            let scheduled_event = scheduled_event.into_inner();

            info!(logger, "Leader scheduled event" ;
                "scheduled at_time" => format!("{:?}", scheduled_event.expected_time),
                "scheduled_at_date" => format!("{}", scheduled_event.date),
            );

            let block = prepare_block(
                &transaction_pool,
                scheduled_event.date,
                blockchain_tip.chain_length().unwrap().next(),
                blockchain_tip.hash().unwrap(),
            );

            let block = match scheduled_event.leader_output {
                LeaderOutput::None => unreachable!("Output::None are supposed to be filtered out"),
                LeaderOutput::Bft(_) => {
                    let leader = leader.read().unwrap();
                    if let Some(ref leader) = &leader.bft_leader {
                        block.make_bft_block(&leader.sig_key)
                    } else {
                        unreachable!("the leader was elected for BFT signing block, we expect it has the signing key")
                    }
                }
                LeaderOutput::GenesisPraos(witness) => {
                    let mut leader = leader.write().unwrap();
                    if let Some(genesis_leader) = &mut leader.genesis_leader {
                        block.make_genesis_praos_block(
                            &genesis_leader.node_id,
                            &mut genesis_leader.sig_key,
                            witness,
                        )
                    } else {
                        unreachable!("the leader was elected for Genesis Praos signing block, we expect it has the signing key")
                    }
                }
            };

            block_message.send(BlockMsg::LeadershipBlock(block));

            future::ok(())
        })
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

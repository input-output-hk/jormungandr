use crate::{
    blockcfg::{
        BlockBuilder, BlockDate, ChainLength, HeaderContentEvalContext, HeaderHash, Leader,
        LeaderOutput, Ledger,
    },
    blockchain::Tip,
    fragment::Pool,
    intercom::BlockMsg,
    leadership::{LeaderSchedule, Leadership},
    secure::enclave::{Enclave, LeaderId},
    utils::async_msg::MessageBox,
};
use chain_core::property::ChainLength as _;
use chain_time::timeframe::TimeFrame;
use slog::Logger;
use std::sync::Arc;
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
    pub leadership: Arc<Leadership>,
    pub time_frame: TimeFrame,
}

pub struct Task {
    logger: Logger,
    leader: LeaderId,
    enclave: Enclave,
    blockchain_tip: Tip,
    epoch_receiver: watch::Receiver<Option<TaskParameters>>,
    fragment_pool: Pool,
    block_message: MessageBox<BlockMsg>,
}

impl Task {
    #[inline]
    pub fn new(
        logger: Logger,
        leader: LeaderId,
        enclave: Enclave,
        blockchain_tip: Tip,
        fragment_pool: Pool,
        epoch_receiver: watch::Receiver<Option<TaskParameters>>,
        block_message: MessageBox<BlockMsg>,
    ) -> Self {
        let logger = Logger::root(
            logger,
            o!(
                ::log::KEY_SUB_TASK => "Leader Task",
                // TODO: add some general context information here (leader alias?)
            ),
        );

        Task {
            logger,
            leader: leader,
            enclave: enclave,
            blockchain_tip,
            fragment_pool,
            epoch_receiver,
            block_message,
        }
    }

    pub fn start(self) -> impl Future<Item = (), Error = ()> {
        let handle_logger = self.logger.clone();
        let crit_logger = self.logger;
        let leader = self.leader;
        let enclave = self.enclave;
        let blockchain_tip = self.blockchain_tip;
        let fragment_pool = self.fragment_pool;
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
                    leader,
                    enclave.clone(),
                    handle_logger.clone(),
                    blockchain_tip.clone(),
                    fragment_pool.clone(),
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
    leader_id: LeaderId,
    enclave: Enclave,
    logger: Logger,
    blockchain_tip: Tip,
    mut fragment_pool: Pool,
    task_parameters: TaskParameters,
) -> impl Future<Item = (), Error = HandleLeadershipError> {
    let schedule = LeaderSchedule::new(logger.clone(), &leader_id, &enclave, &task_parameters);

    schedule
        .map_err(|err| HandleLeadershipError::Schedule { source: err })
        .for_each(move |scheduled_event| {
            let scheduled_event = scheduled_event.into_inner();

            info!(logger, "Leader scheduled event" ;
                "scheduled at_time" => format!("{:?}", scheduled_event.expected_time),
                "scheduled_at_date" => format!("{}", scheduled_event.leader_output.date),
            );

            let block = prepare_block(
                &mut fragment_pool,
                blockchain_tip.ledger().unwrap().clone(),
                &task_parameters.leadership,
                scheduled_event.leader_output.date,
                blockchain_tip.chain_length().unwrap().next(),
                blockchain_tip.hash().unwrap(),
            );

            let block = enclave.create_block(block, scheduled_event.leader_output);

            block_message
                .try_send(BlockMsg::LeadershipBlock(block))
                .unwrap();

            future::ok(())
        })
}

fn prepare_block(
    fragment_pool: &mut Pool,
    ledger: Ledger,
    leadership: &Leadership,
    date: BlockDate,
    chain_length: ChainLength,
    parent_id: HeaderHash,
) -> BlockBuilder {
    use crate::fragment::selection::{FragmentSelectionAlgorithm as _, OldestFirst};

    let selection_algorithm = OldestFirst::new(250 /* TODO!! */);
    let metadata = HeaderContentEvalContext {
        block_date: date,
        chain_length,
        nonce: None,
    };
    let ledger_params = leadership.ledger_parameters().clone();

    let mut bb = fragment_pool
        .select(ledger, metadata, ledger_params, selection_algorithm)
        .wait()
        .unwrap()
        .finalize();

    bb.date(date).parent(parent_id).chain_length(chain_length);

    bb
}

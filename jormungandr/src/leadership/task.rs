use crate::{
    blockcfg::{
        BlockBuilder, BlockDate, ChainLength, HeaderContentEvalContext, HeaderHash, Ledger,
    },
    blockchain::Tip,
    fragment::Pool,
    intercom::BlockMsg,
    leadership::{LeaderSchedule, Leadership},
    secure::enclave::{Enclave, LeaderId},
    stats_counter::StatsCounter,
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
    leadership_params: HandleLeadershipParams,
    epoch_receiver: watch::Receiver<Option<TaskParameters>>,
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
        stats_counter: StatsCounter,
    ) -> Self {
        let logger = Logger::root(
            logger,
            o!(
                ::log::KEY_SUB_TASK => "Leader Task",
                // TODO: add some general context information here (leader alias?)
            ),
        );

        Task {
            leadership_params: HandleLeadershipParams {
                logger,
                leader,
                enclave,
                blockchain_tip,
                fragment_pool,
                block_message,
                stats_counter,
            },
            epoch_receiver,
        }
    }

    pub fn start(self) -> impl Future<Item = (), Error = ()> {
        let crit_logger = self.leadership_params.logger.clone();
        let leadership_params = self.leadership_params;
        self.epoch_receiver
            .map_err(|error| TaskError::LeadershipReceiver {
                extra: format!("{}", error),
            })
            // filter_map so we don't have to do the pattern match on `Option::Nothing`.
            .filter_map(|task_parameters| task_parameters)
            .for_each(move |task_parameters| {
                handle_leadership(leadership_params.clone(), task_parameters)
                .map_err(|error| {
                    TaskError::LeadershipHandle { source: error }
                })
            })
            .map_err(move |error| {
                crit!(crit_logger, "critical error in the Leader task" ; "reason" => error.to_string())
            })
    }
}

#[derive(Clone)]
struct HandleLeadershipParams {
    logger: Logger,
    leader: LeaderId,
    enclave: Enclave,
    blockchain_tip: Tip,
    fragment_pool: Pool,
    block_message: MessageBox<BlockMsg>,
    stats_counter: StatsCounter,
}

/// function that will run for the length of the Epoch associated
/// to the given leadership
///
fn handle_leadership(
    leadership_params: HandleLeadershipParams,
    task_parameters: TaskParameters,
) -> impl Future<Item = (), Error = HandleLeadershipError> {
    let HandleLeadershipParams {
        logger,
        leader,
        enclave,
        mut block_message,
        mut fragment_pool,
        blockchain_tip,
        stats_counter,
    } = leadership_params;
    let schedule = LeaderSchedule::new(logger.clone(), &leader, &enclave, &task_parameters);

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
            stats_counter.set_slot_start_time(scheduled_event.expected_time);
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

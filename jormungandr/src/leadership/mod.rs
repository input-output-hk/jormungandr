//! new module to prepare for the new leadership scheduling of blocks
//!
//! here we need to take into consideration that won't have access to the
//! cryptographic objects of the leader: they will be executed in a secure
//! enclave.
//!
//! ## data structures
//!
//! We need to separate our components as following:
//!
//! 1. the enclave:
//!     * upon receiving the necessary parameters, it will return a schedule
//!       when it should be elected to create a block;
//!     * upon receiving the necessary parameters, it will create the
//!       proof to finalize the creation of a block;
//! 2. the schedule:
//!     * it holds the schedules for a given epoch
//!     * we can query it to get a list of schedule for the REST API (useful to
//!       have information when the node is expected to create blocks);
//!     * optional but useful: have a way to update if a schedule has been
//!       executed (and what time);
//!     * optional: have a way for the blockchain task to update
//!       the schedule to know if the scheduled block as been accepted in the
//!       branch;
//!
//! The enclave is not yet implemented, but we will need to separate the crypto
//! from the representation here.
//!
//! ## workflow
//!
//! The flow process will work as follow:
//!
//! 1. the leadership module will receive a new event to create prepare a leadership
//!    schedule; It will only includes the `Leadership` object from chain_lib and the
//!    `TimeFrame` active for the future blocks to come;
//! 2. upon receiving these data, it will query the **enclave** to know the list of expected
//!    scheduled leader elections; (this part may require heavy cryptographic computation,
//!    we may want to split this part into incrementally long queries);
//! 3. once the schedule is retrieved (even partially) we can start waiting for the appropriate
//!    time to create a new block (to run block fragment selection) and ask the enclave to sign
//!    the block;
//! 4. once a block is ready we need to send it to the blockchain task to process it and update
//!    the blockchain.
//!
//! ## how and when to trigger a new leadership event
//!
//! The blockchain module has the material to create the new leadership parameters
//! for a given epoch (the `Leadership` object and the time frame). It needs to send
//! the appropriate data when necessary.
//!
//! 2 ways to trigger a new leadership schedule from the blockchain module:
//!
//! 1. the blockchain detects an epoch transition,
//! 2. the leadership sent an end of epoch signal to the blockchain;
//!
//! Now doing so we may trigger the same leader schedule twice. We will need to make sure
//! we don't duplicate the work everywhere.
//!

mod enclave;
mod logs;
mod schedule;

pub use self::enclave::{Enclave, Error as EnclaveError, LeaderEvent};
pub use self::logs::{LeadershipLogHandle, Logs};
pub use self::schedule::{Schedule, Schedules};
use crate::{
    blockcfg::{BlockBuilder, BlockDate, HeaderContentEvalContext, Leadership, LedgerParameters},
    blockchain::Branch,
    fragment,
    intercom::BlockMsg,
    utils::{async_msg::MessageBox, task::TokioServiceInfo},
};
use chain_time::{
    era::{EpochPosition, EpochSlotOffset},
    TimeFrame,
};
use std::sync::Arc;
use tokio::{prelude::*, sync::mpsc};

error_chain! {
    errors {
        ScheduleError {
            description("error while polling for scheduled events"),
        }
        NewEpochToScheduleReceiverError {
            description("cannot receive new epoch to schedule notification "),
        }
        FragmentSelectionFailed {
            description("fragment selection failed")
        }
        Enclave {
            description("error while querying the enclave")
        }
        CannotSendLeadershipBlock {
            description("Cannot send the leadership's new created block")
        }
    }
}

pub struct NewEpochToSchedule {
    pub new_schedule: Arc<Leadership>,
    pub new_parameters: Arc<LedgerParameters>,
    pub time_frame: TimeFrame,
}

pub struct LeadershipModule {
    logs: Logs,
    service_info: TokioServiceInfo,
    enclave: Enclave,
    fragment_pool: fragment::Pool,
    tip: Branch,
    block_message: MessageBox<BlockMsg>,
}

impl LeadershipModule {
    fn handle_schedule(&self, schedule: Schedule) {
        let logger = self.service_info.logger().new(o!("leader" => schedule.leader_event().id.to_string(), "date" => schedule.leader_event().date.to_string()));
        let fragment_pool = self.fragment_pool.clone();
        let tip = self.tip.clone();
        let enclave = self.enclave.clone();
        let leader_event: LeaderEvent = schedule.leader_event;
        let date = leader_event.date.clone();
        let ledger_parameters = schedule.epoch_ledger_parameters;
        let sender = self.block_message.clone();
        let log_awake = schedule.log.mark_wake();
        let log_finish = schedule.log.mark_finished();

        self.service_info.spawn(
            log_awake
                .map_err(|()| unreachable!())
                .and_then(move |()| {
                    info!(logger, "leader event starting");

                    prepare_block(fragment_pool, date, tip, ledger_parameters)
                })
                .and_then(move |bb| {
                    enclave
                        .query_block_finalize(bb, leader_event)
                        .map_err(|_| unimplemented!())
                })
                .and_then(|block| {
                    sender
                        .send(BlockMsg::LeadershipBlock(block))
                        .map_err(|_send_error| ErrorKind::CannotSendLeadershipBlock.into())
                })
                .and_then(|_: MessageBox<BlockMsg>| log_finish.map_err(|()| unreachable!()))
                .map_err(|_: Error| unimplemented!()),
        );
    }

    fn handle_new_epoch_event(
        self,
        scheduler: Schedules,
        new_epoch_event: NewEpochToSchedule,
    ) -> impl Future<Item = (Self, Schedules), Error = Error> {
        let leadership = new_epoch_event.new_schedule;
        let epoch_parameters = new_epoch_event.new_parameters;
        let era = leadership.era().clone();
        let epoch = leadership.epoch();
        let time_frame = new_epoch_event.time_frame;
        let logs = self.logs.clone();

        let slot_start = 0;
        let nb_slots = era.slots_per_epoch();

        let logger = self.service_info.logger().new(o!("epoch" => epoch));

        debug!(logger, "handling new epoch event");

        self.enclave
            .query_schedules(leadership.clone(), slot_start, nb_slots)
            .map_err(|e| Error::with_chain(e, ErrorKind::Enclave))
            .and_then(move |schedules| {
                stream::iter_ok::<_, Error>(schedules).fold(
                    scheduler,
                    move |scheduler, schedule| {
                        let slot = era.from_era_to_slot(EpochPosition {
                            epoch: chain_time::Epoch(schedule.date.epoch),
                            slot: EpochSlotOffset(schedule.date.slot_id),
                        });
                        let slot_system_time = time_frame
                            .slot_to_systemtime(slot)
                            .expect("The slot should always be in the given time frame here");

                        debug!(logger, "registering new leader event";
                            "leader"     => schedule.id.to_string(),
                            "block date" => schedule.date.to_string()
                        );

                        scheduler
                            .schedule(
                                logs.clone(),
                                leadership.clone(),
                                epoch_parameters.clone(),
                                slot_system_time.into(),
                                schedule,
                            )
                            .map_err(|()| Error::from("error while adding a new schedule"))
                    },
                )
            })
            .map(|scheduler| (self, scheduler))
    }

    pub fn start(
        service_info: TokioServiceInfo,
        logs: Logs,
        enclave: Enclave,
        fragment_pool: fragment::Pool,
        tip_branch: Branch,
        new_epoch_events: mpsc::Receiver<NewEpochToSchedule>,
        block_message: MessageBox<BlockMsg>,
    ) -> impl Future<Item = (), Error = Error> {
        let scheduler_future = Schedules::new().into_future();
        let new_epoch_future = new_epoch_events.into_future();

        let leadership_module = LeadershipModule {
            logs,
            service_info,
            enclave,
            fragment_pool,
            tip: tip_branch,
            block_message,
        };

        future::loop_fn(
            (leadership_module, scheduler_future, new_epoch_future),
            |(leadership_module, scheduler_future, new_epoch_future)| {
                scheduler_future
                    .select2(new_epoch_future)
                    .map_err(|either| match either {
                        future::Either::A(((error, _scheduler), _new_epoch_events)) => {
                            Error::with_chain(error, ErrorKind::ScheduleError)
                        }
                        future::Either::B(((error, _new_epoch_events), _scheduler)) => {
                            Error::with_chain(error, ErrorKind::NewEpochToScheduleReceiverError)
                        }
                    })
                    .and_then(move |either| {
                        match either {
                            future::Either::A(((schedule, schedules), new_epoch_future)) => {
                                let schedule = schedule.expect(
                                    "delay queue should always be NotReady if no more schedule",
                                );
                                let scheduler_future = schedules.into_future();

                                leadership_module.handle_schedule(schedule.into_inner());

                                future::Either::A(future::ok((
                                    leadership_module,
                                    scheduler_future,
                                    new_epoch_future,
                                )))
                            }
                            future::Either::B((
                                (new_epoch_event, new_epoch_events),
                                scheduler_future,
                            )) => {
                                let new_epoch_event =
                                    new_epoch_event.expect("Expect the event to not close");

                                // the stream didn't yield an element so we can retrieve the inner schedule here
                                let schedules = scheduler_future.into_inner().unwrap();

                                future::Either::B(
                                    leadership_module
                                        .handle_new_epoch_event(schedules, new_epoch_event)
                                        .map(move |(leadership_module, schedules)| {
                                            (
                                                leadership_module,
                                                schedules.into_future(),
                                                new_epoch_events.into_future(),
                                            )
                                        }),
                                )
                            }
                        }
                    })
                    // for now we continue the loop forever (until kill/crash)
                    // we can at some point connect this to the `utils::task::Task`
                    // input feature to `Loop::Break` on receiving a shutdown
                    // instruction
                    .map(future::Loop::Continue)
            },
        )
    }
}

fn prepare_block(
    mut fragment_pool: fragment::Pool,
    date: BlockDate,
    tip: Branch,
    epoch_parameters: Arc<LedgerParameters>,
) -> impl Future<Item = BlockBuilder, Error = Error> {
    use crate::fragment::selection::{FragmentSelectionAlgorithm as _, OldestFirst};

    let selection_algorithm = OldestFirst::new(250 /* TODO!! */);

    tip.get_ref()
        .map_err(|_: std::convert::Infallible| unreachable!())
        .and_then(move |tip_reference| {
            use chain_core::property::ChainLength as _;

            let parent_id = tip_reference.hash().clone();
            let chain_length = tip_reference.chain_length().next();
            let ledger = tip_reference.ledger();

            let metadata = HeaderContentEvalContext {
                block_date: date,
                chain_length,
                nonce: None,
            };

            fragment_pool
                .select(
                    ledger.as_ref().clone(),
                    metadata,
                    epoch_parameters.as_ref().clone(),
                    selection_algorithm,
                )
                .map(|selection_algorithm| selection_algorithm.finalize())
                .map(move |mut bb| {
                    bb.date(date).parent(parent_id).chain_length(chain_length);
                    bb
                })
                .map_err(|()| ErrorKind::FragmentSelectionFailed.into())
        })
}

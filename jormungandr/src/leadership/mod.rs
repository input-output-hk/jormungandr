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
    blockcfg::{
        Block, BlockDate, BlockVersion, Contents, Epoch, HeaderBuilderNew,
        HeaderContentEvalContext, LeaderOutput, Leadership, Ledger, LedgerParameters,
    },
    blockchain::Tip,
    fragment,
    intercom::BlockMsg,
    utils::{async_msg::MessageBox, task::TokioServiceInfo},
};
use chain_time::{
    era::{EpochPosition, EpochSlotOffset},
    TimeFrame,
};
use jormungandr_lib::time::SystemTime;
use std::{sync::Arc, time::Duration};
use tokio::{
    prelude::*,
    sync::mpsc,
    timer::{Delay, Interval},
};

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
    tip: Tip,
    block_message: MessageBox<BlockMsg>,
    garbage_collection_interval: Duration,
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

        let tip_reference = tip
            .get_ref()
            .map_err(|_: std::convert::Infallible| unreachable!());

        self.service_info.spawn(
            log_awake
                .map_err(|()| unreachable!())
                .join(tip_reference)
                .and_then(move |((), tip_reference)| {
                    info!(logger, "leader event starting");

                    let parent_id = tip_reference.hash().clone();
                    let chain_length = tip_reference.chain_length().increase();
                    let ledger = tip_reference.ledger();

                    let eval_context = HeaderContentEvalContext {
                        block_date: date,
                        chain_length,
                        nonce: None,
                    };
                    let next = (parent_id, chain_length, date);
                    prepare_block(fragment_pool, eval_context, ledger, ledger_parameters)
                        .join(future::ok(next))
                })
                .and_then(move |(contents, (parent_id, chain_length, date))| {
                    let ver = match leader_event.output {
                        LeaderOutput::None => BlockVersion::Genesis,
                        LeaderOutput::Bft(_) => BlockVersion::Ed25519Signed,
                        LeaderOutput::GenesisPraos(..) => BlockVersion::KesVrfproof,
                    };
                    let hdr_builder = HeaderBuilderNew::new(ver, &contents)
                        .set_parent(&parent_id, chain_length)
                        .set_date(date);
                    match leader_event.output {
                        LeaderOutput::None => {
                            let header = hdr_builder.to_unsigned_header().unwrap().generalize();
                            future::Either::A(future::ok(Block { header, contents }))
                        }
                        LeaderOutput::Bft(leader_id) => {
                            let final_builder = hdr_builder
                                .to_bft_builder()
                                .unwrap()
                                .set_consensus_data(&leader_id);
                            future::Either::B(future::Either::A(
                                enclave
                                    .query_header_bft_finalize(final_builder, leader_event.id)
                                    .map(|h| Block {
                                        header: h.generalize(),
                                        contents,
                                    })
                                    .map_err(|_| unimplemented!()),
                            ))
                        }
                        LeaderOutput::GenesisPraos(node_id, vrfproof) => {
                            let final_builder = hdr_builder
                                .to_genesis_praos_builder()
                                .unwrap()
                                .set_consensus_data(&node_id, &vrfproof.into());
                            future::Either::B(future::Either::B(
                                enclave
                                    .query_header_genesis_praos_finalize(
                                        final_builder,
                                        leader_event.id,
                                    )
                                    .map(|h| Block {
                                        header: h.generalize(),
                                        contents,
                                    })
                                    .map_err(|_| unimplemented!()),
                            ))
                        }
                    }
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

    fn spawn_end_of_epoch(&self, time_frame: &TimeFrame, epoch: Epoch, slot: chain_time::Slot) {
        let slot_system_time = time_frame
            .slot_to_systemtime(slot)
            .expect("The slot should always be in the given time frame here");

        let now = std::time::SystemTime::now();

        let duration = match slot_system_time.duration_since(now) {
            Err(error) => {
                let logger = self.service_info.logger().new(o!(
                    "now" => SystemTime::from(now).to_string(),
                    "slot_system_time" => SystemTime::from(slot_system_time).to_string(),
                    "reason" => error.to_string(),
                    "epoch" => epoch,
                ));
                if let Ok(duration) = now.duration_since(slot_system_time) {
                    crit!(
                        logger,
                        "system recorded a {}s delay. This could be due to a system suspension or hibernation, in order not to miss out on future leader elections please prevent your system from suspending or hibernating.",
                        duration.as_secs(),
                    )
                }

                unimplemented!(
                    r###"The system just failed to compute an appropriate instant.
This could be due to a system suspension or hibernation, in order not to miss out on future
leader elections please prevent your system from suspending or hibernating.
"###
                );
            }
            Ok(duration) => duration,
        };
        let scheduled_time = std::time::Instant::now() + duration;

        let sa: SystemTime = (now + duration).into();
        debug!(self.service_info.logger(), "scheduling new end of epoch"
        ; "scheduled at" => sa.to_string());

        let logger1 = self.service_info.logger().new(o!());
        let logger2 = self.service_info.logger().new(o!());
        let block_message = self.block_message.clone();

        self.service_info.spawn(
            Delay::new(scheduled_time)
                .map_err(move |err| crit!(logger1, "cannot reschedule future epoch" ; "reason" => err.to_string()))
                .and_then(move |()| {
                    info!(logger2, "end of epoch");
                    block_message.send(BlockMsg::LeadershipExpectEndOfEpoch(epoch))
                        .map_err(move |_| {
                            crit!(logger2, "cannot reschedule future epoch" ; "reason" => "cannot send the BlockMsg end of epoch")
                        })
                        .map(|_| ())
                })
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

        let current_slot = time_frame.slot_at(&std::time::SystemTime::now()).unwrap();
        let within_era = era.from_slot_to_era(current_slot).unwrap();

        let slot_start = within_era.slot.0;
        let nb_slots = era.slots_per_epoch() - slot_start;

        let logger = self.service_info.logger().new(o!("epoch" => epoch));

        debug!(logger, "handling new epoch event";
            "slot start" => slot_start,
            "nb_slots" => nb_slots,
        );

        self.spawn_end_of_epoch(
            &time_frame,
            epoch,
            era.from_era_to_slot(EpochPosition {
                epoch: chain_time::Epoch(epoch + 1),
                slot: EpochSlotOffset(0),
            }),
        );

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
                        let slot_system_time: SystemTime = time_frame
                            .slot_to_systemtime(slot)
                            .expect("The slot should always be in the given time frame here")
                            .into();

                        let now = SystemTime::now();

                        if slot_system_time <= now {
                            debug!(logger, "ignoring new leader event";
                                "leader"     => schedule.id.to_string(),
                                "block date" => schedule.date.to_string(),
                                "scheduled_at" => slot_system_time.to_string(),
                                "now" => now.to_string(),
                            );
                            future::Either::A(future::ok(scheduler))
                        } else {
                            debug!(logger, "registering new leader event";
                                "leader"     => schedule.id.to_string(),
                                "block date" => schedule.date.to_string(),
                                "scheduled_at" => slot_system_time.to_string(),
                            );

                            future::Either::B(
                                scheduler
                                    .schedule(
                                        logs.clone(),
                                        leadership.clone(),
                                        epoch_parameters.clone(),
                                        slot_system_time,
                                        schedule,
                                    )
                                    .map_err(|()| Error::from("error while adding a new schedule")),
                            )
                        }
                    },
                )
            })
            .map(|scheduler| (self, scheduler))
    }

    fn spawn_log_purge(&self) -> impl Future<Item = (), Error = ()> {
        let mut logs = self.logs.clone();
        let garbage_collection_interval = self.garbage_collection_interval;
        let logger = self
            .service_info
            .logger()
            .new(o!("sub task" => "garbage collection"));
        let error_logger = logger.clone();
        Interval::new_interval(garbage_collection_interval)
            .for_each(move |_instant| {
                debug!(logger, "garbage collect entries in the logs");
                logs.poll_purge()
            })
            .map_err(move |error| {
                error!(error_logger, "Cannot run the garbage collection" ; "reason" => error.to_string());
            })
    }

    pub fn start(
        service_info: TokioServiceInfo,
        logs: Logs,
        garbage_collection_interval: Duration,
        enclave: Enclave,
        fragment_pool: fragment::Pool,
        tip_branch: Tip,
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
            garbage_collection_interval,
        };

        leadership_module
            .service_info
            .spawn(leadership_module.spawn_log_purge());

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
                                if let Some(schedule) = schedule {
                                    leadership_module.handle_schedule(schedule.into_inner());
                                } else {
                                    unreachable!(
                                        "Schedules stream either returns item or stay Async::NoReady"
                                    )
                                }

                                let scheduler_future = schedules.into_future();

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
    eval_context: HeaderContentEvalContext,
    ledger: &Arc<Ledger>,
    epoch_parameters: Arc<LedgerParameters>,
) -> impl Future<Item = Contents, Error = Error> {
    use crate::fragment::selection::{FragmentSelectionAlgorithm as _, OldestFirst};

    let selection_algorithm = OldestFirst::new(250 /* TODO!! */);
    fragment_pool
        .select(
            ledger.as_ref().clone(),
            eval_context,
            epoch_parameters.as_ref().clone(),
            selection_algorithm,
        )
        .map(|selection_algorithm| selection_algorithm.finalize())
        .map_err(|()| ErrorKind::FragmentSelectionFailed.into())
}

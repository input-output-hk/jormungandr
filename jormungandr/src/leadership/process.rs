use crate::{
    blockcfg::{
        Block, BlockDate, BlockVersion, Contents, HeaderBuilderNew, LeaderOutput, Leadership,
        Ledger, LedgerParameters,
    },
    blockchain::{new_epoch_leadership_from, Ref, Tip},
    intercom::{unary_reply, BlockMsg, Error as IntercomError, TransactionMsg},
    leadership::{
        enclave::{Enclave, EnclaveError, LeaderEvent},
        LeadershipLogHandle, Logs,
    },
    utils::{async_msg::MessageBox, task::TokioServiceInfo},
};
use chain_time::{
    era::{EpochPosition, EpochSlotOffset},
    Epoch, Slot,
};
use futures03::{future::TryFutureExt, sink::SinkExt};
use jormungandr_lib::{
    interfaces::{LeadershipLog, LeadershipLogStatus},
    time::SystemTime,
};
use slog::Logger;
use std::{collections::VecDeque, sync::Arc, time::Instant};
use thiserror::Error;
use tokio02::time::{delay_until, timeout_at, Instant as TokioInstant};

#[derive(Error, Debug)]
pub enum LeadershipError {
    #[error("The blockchain Timeline hasn't started yet")]
    TooEarlyForTimeFrame {
        time: jormungandr_lib::time::SystemTime,
        // TODO: it would be nice to get the starting time
        //       of the time frame to report appropriate error
    },

    #[error("Cannot query enclave for leader schedules")]
    CannotScheduleWithEnclave {
        #[source]
        source: EnclaveError,
    },

    #[error("fragment selection failed")]
    FragmentSelectionFailed(#[from] IntercomError),

    #[error("Error while connecting to the fragment pool to query fragments for block")]
    CannotConnectToFragmentPool,

    #[error("Cannot send the leadership block to the blockchain module")]
    CannotSendLeadershipBlock,

    #[error("Cannot update the leadership logs")]
    CannotUpdateLogs,
}

struct Entry {
    event: LeaderEvent,
    log: LeadershipLogHandle,
}

#[derive(Default)]
struct Schedule {
    entries: VecDeque<Entry>,
}

pub struct Module {
    schedule: Schedule,
    service_info: TokioServiceInfo,
    logs: Logs,
    tip_ref: Arc<Ref>,
    tip: Tip,
    pool: MessageBox<TransactionMsg>,
    enclave: Enclave,
    block_message: MessageBox<BlockMsg>,
}

impl Module {
    pub async fn new(
        service_info: TokioServiceInfo,
        logs: Logs,
        tip: Tip,
        pool: MessageBox<TransactionMsg>,
        enclave: Enclave,
        block_message: MessageBox<BlockMsg>,
    ) -> Result<Self, LeadershipError> {
        let tip_ref = tip.get_ref().await;

        Ok(Self {
            schedule: Schedule::default(),
            service_info,
            logs,
            tip_ref,
            tip,
            pool,
            enclave,
            block_message,
        })
    }

    pub async fn run(self) -> Result<(), LeadershipError> {
        let mut module = self;
        loop {
            module = module.step().await?;
        }
    }

    async fn step(self) -> Result<Self, LeadershipError> {
        self.action().await?.wait().await
    }

    fn current_slot(&self) -> Result<Slot, LeadershipError> {
        let time_frame = self.tip_ref.time_frame();

        let now = SystemTime::now();
        if let Some(current_slot) = time_frame.slot_at(now.as_ref()) {
            Ok(current_slot)
        } else {
            // in the current blockchain settings this can only happen if we started
            // the called this function before the block0 start date time.

            Err(LeadershipError::TooEarlyForTimeFrame { time: now.into() })
        }
    }

    fn current_slot_position(&self) -> Result<EpochPosition, LeadershipError> {
        let leadership = self.tip_ref.epoch_leadership_schedule();
        let era = leadership.era();

        let current_slot = self.current_slot()?;
        if let Some(current_position) = era.from_slot_to_era(current_slot) {
            Ok(current_position)
        } else {
            // it appears the `current_slot` was set **before** the beginning
            // of the era. This should not be possible because we took it from
            // the parameter of the blockchain.

            unreachable!()
        }
    }

    /// this function compute when the next epoch will start, next epoch
    /// from the local system time point of view. Meaning this is not the
    /// epoch of the current tip
    fn next_epoch_time(&self) -> Result<SystemTime, LeadershipError> {
        let current_position = self.current_slot_position()?;
        let epoch = Epoch(current_position.epoch.0 + 1);
        let slot = EpochSlotOffset(0);

        Ok(self.slot_time(epoch, slot))
    }

    fn next_epoch_instant(&self) -> Result<Instant, LeadershipError> {
        let next_epoch_time = self.next_epoch_time()?;

        match next_epoch_time
            .as_ref()
            .duration_since(SystemTime::now().into())
        {
            Err(err) => {
                // only possible if `next_epoch_time` is earlier than now. I.e. if the next
                // epoch is in the past.

                unreachable!(
                    "next epoch is in the past. This is not possible, but it seems it append. {}",
                    err
                )
            }
            Ok(duration) => Ok(Instant::now() + duration),
        }
    }

    fn slot_time(&self, epoch: Epoch, slot: EpochSlotOffset) -> SystemTime {
        let leadership = self.tip_ref.epoch_leadership_schedule();
        let time_frame = self.tip_ref.time_frame();
        let era = leadership.era();

        let slot = era.from_era_to_slot(EpochPosition { epoch, slot });
        if let Some(slot_time) = time_frame.slot_to_systemtime(slot) {
            slot_time.into()
        } else {
            // the slot is referring to a time before the time_frame.
            // this should not be possible.

            unreachable!()
        }
    }

    // get the slot time of the given event, this is the start point
    // where the slot time is valid
    fn event_slot_time(&self, event: &LeaderEvent) -> SystemTime {
        let epoch = Epoch(event.date.epoch);
        let slot = EpochSlotOffset(event.date.slot_id);
        self.slot_time(epoch, slot)
    }

    // gives the slot time of the following slot, the slot that follow directly
    // the given event, being exactly the strict upper bound
    //
    // if slot date is `E.X` (E = Epoch, X = current epoch slot offset)
    // the function will return the schedule time for `E.(X+1)`.
    fn event_following_slot_time(&self, event: &LeaderEvent) -> SystemTime {
        let leadership = self.tip_ref.epoch_leadership_schedule();
        let era = leadership.era();

        let epoch = Epoch(event.date.epoch);
        let slot = EpochSlotOffset(event.date.slot_id + 1);

        if era.slots_per_epoch() <= slot.0 {
            self.slot_time(Epoch(epoch.0 + 1), EpochSlotOffset(0))
        } else {
            self.slot_time(epoch, slot)
        }
    }

    fn slot_instant(&self, epoch: Epoch, slot: EpochSlotOffset) -> Option<Instant> {
        let slot_time = self.slot_time(epoch, slot);

        match slot_time.as_ref().duration_since(SystemTime::now().into()) {
            Err(_err) => {
                // this may happen if the epoch/slot is long gone
                None
            }
            Ok(duration) => Some(Instant::now() + duration),
        }
    }

    async fn wait(mut self) -> Result<Self, LeadershipError> {
        let deadline = self.wait_peek_deadline()?;
        delay_until(TokioInstant::from_std(deadline)).await;
        let tip = self.tip.clone();
        self.tip_ref = tip.get_ref().await;
        Ok(self)
    }

    fn wait_peek_deadline(&self) -> Result<Instant, LeadershipError> {
        match self.schedule.peek() {
            None => {
                // the schedule is empty we were in the _action_ mode, so that means
                // there is no other schedule to have for the current epoch. Better
                // wait for the next epoch

                debug!(
                    self.service_info.logger(),
                    "no item scheduled, waiting for next epoch"
                );
                self.next_epoch_instant()
            }
            Some(entry) => {
                let logger = self.service_info.logger().new(o!(
                    "event_date" => entry.event.date.to_string(),
                    "leader_id" => entry.event.id.to_string(),
                ));
                if let Some(instant) = entry.instant(&self)? {
                    debug!(logger, "awaiting");
                    Ok(instant)
                } else {
                    // if the entry didn't have a valid epoch instant it means
                    // we are looking at passed entry already or it is happening
                    // now. so don't wait any further
                    debug!(logger, "scheduled time for event was missed");
                    Ok(Instant::now())
                }
            }
        }
    }
    async fn action(mut self) -> Result<Self, LeadershipError> {
        match self.schedule.pop() {
            None => self.action_schedule().await,
            Some(entry) => self.action_entry(entry).await,
        }
    }

    async fn action_entry(self, entry: Entry) -> Result<Self, LeadershipError> {
        let end_log = entry.log.clone();
        entry.log.mark_wake().await;
        let module = self.action_run_entry(entry).await?;
        end_log.mark_finished().await;
        Ok(module)
    }

    async fn action_run_entry(self, entry: Entry) -> Result<Self, LeadershipError> {
        let now = SystemTime::now();
        let event_start = self.event_slot_time(&entry.event);
        let event_end = self.event_following_slot_time(&entry.event);

        let logger = self.service_info.logger().new(o!(
            "leader_id" => entry.event.id.to_string(),
            "event_date" => entry.event.date.to_string(),
            "event_start" => event_start.to_string(),
            "event_end" => event_end.to_string(),
        ));

        if too_late(now, event_end) {
            // the event happened out of bounds, ignore it and move to the next one
            error!(
                logger,
                "Eek... Too late, we missed an event schedule, system time might be off?"
            );

            entry
                .log
                .set_status(LeadershipLogStatus::Rejected {
                    reason: "Missed the deadline to compute the schedule".to_owned(),
                })
                .await;

            Ok(self)
        } else {
            let right_time = entry.instant(&self)?;

            if let Some(right_time) = right_time {
                warn!(
                    logger,
                    "system woke a bit early for the event, delaying until right time."
                );

                // await the right_time before starting the action
                delay_until(TokioInstant::from_std(right_time)).await;
                self.action_run_entry_in_bound(entry, logger, event_end)
                    .await
            } else {
                // because we checked that the entry's slot was below the current
                // time, if we cannot compute the _right_time_ it means the time
                // is just starting now to be correct. So it's okay to start
                // running it now still
                self.action_run_entry_in_bound(entry, logger, event_end)
                    .await
            }
        }
    }

    async fn action_run_entry_in_bound(
        self,
        entry: Entry,
        logger: Logger,
        event_end: SystemTime,
    ) -> Result<Self, LeadershipError> {
        let event_logs = entry.log.clone();
        let now = SystemTime::now();

        // we can safely unwrap here as we just proved that `now <= event_end`
        // so that `now` is earlier to `event_end`.
        //
        // This gives us the remaining time to the execute the
        // block building (including block selection) and to submit the block
        // to the network.
        let remaining_time = event_end
            .duration_since(now)
            .expect("event end in the future");
        let deadline = Instant::now() + remaining_time.into();

        let logger = logger.new(o!(
            "event_remaining_time" => jormungandr_lib::time::Duration::from(remaining_time).to_string()
        ));

        info!(logger, "Leader event started");

        let timed_out_log = logger.clone();

        let res = timeout_at(
            TokioInstant::from_std(deadline),
            self.action_run_entry_build_block(entry, logger),
        )
        .await;

        match res {
            Ok(future_res) => future_res,
            Err(timeout_error) => {
                error!(timed_out_log, "Eek... took too long to process the event..." ; "reason" => %timeout_error);
                event_logs
                    .set_status(LeadershipLogStatus::Rejected {
                        reason: "Failed to compute the schedule within time boundaries".to_owned(),
                    })
                    .await;
                Ok(())
            }
        }.map(|()| self)
    }

    async fn action_run_entry_build_block(
        &self,
        entry: Entry,
        logger: Logger,
    ) -> Result<(), LeadershipError> {
        let event = entry.event;
        let event_logs = entry.log;

        let enclave = self.enclave.clone();
        let sender = self.block_message.clone();
        let pool = self.pool.clone();

        let (parent_id, chain_length, ledger, ledger_parameters) = if self.tip_ref.block_date()
            < event.date
        {
            (
                self.tip_ref.hash(),
                self.tip_ref.chain_length().increase(),
                self.tip_ref.ledger(),
                Arc::clone(self.tip_ref.epoch_ledger_parameters()),
            )
        } else {
            // it appears we are either competing against another stake pool for the same
            // slot or we are a bit behind schedule
            //
            // TODO: check up to a certain distance a valid block to use as parent
            //       for now we will simply exit early
            //
            // * reminder that there is a timeout
            // * jumping epoch is might not be acceptable

            warn!(
                logger,
                "It appears the node is running a bit behind schedule, system time might be off?"
            );

            event_logs.set_status(
                    LeadershipLogStatus::Rejected {
                        reason: "Not computing this schedule because of invalid state against the network blockchain".to_owned()
                    }
                ).await;

            return Ok(());
        };

        let contents =
            prepare_block(pool, event.date, ledger, ledger_parameters, logger.clone()).await?;

        let event_logs_error = event_logs.clone();
        let signing = {
            let ver = match event.output {
                LeaderOutput::None => BlockVersion::Genesis,
                LeaderOutput::Bft(_) => BlockVersion::Ed25519Signed,
                LeaderOutput::GenesisPraos(..) => BlockVersion::KesVrfproof,
            };

            let hdr_builder = HeaderBuilderNew::new(ver, &contents)
                .set_parent(&parent_id, chain_length)
                .set_date(event.date);

            match event.output {
                LeaderOutput::None => {
                    let header = hdr_builder
                        .into_unsigned_header()
                        .expect("Valid Header Builder")
                        .generalize();
                    Ok(Some(Block { header, contents }))
                }
                LeaderOutput::Bft(leader_id) => {
                    let final_builder = hdr_builder
                        .into_bft_builder()
                        .expect("Valid Header Builder")
                        .set_consensus_data(&leader_id);
                    enclave
                        .query_header_bft_finalize(final_builder, event.id)
                        .map_ok(|h| {
                            Some(Block {
                                header: h.generalize(),
                                contents,
                            })
                        })
                        .or_else(|e| async move {
                            event_logs_error
                                .set_status(LeadershipLogStatus::Rejected {
                                    reason: format!("Cannot sign the block: {}", e),
                                })
                                .await;
                            Ok(None)
                        })
                        .await
                }
                LeaderOutput::GenesisPraos(node_id, vrfproof) => {
                    let final_builder = hdr_builder
                        .into_genesis_praos_builder()
                        .expect("Valid Header Builder")
                        .set_consensus_data(&node_id, &vrfproof.into());
                    enclave
                        .query_header_genesis_praos_finalize(final_builder, event.id)
                        .map_ok(|h| {
                            Some(Block {
                                header: h.generalize(),
                                contents,
                            })
                        })
                        .or_else(|e| async move {
                            event_logs_error
                                .set_status(LeadershipLogStatus::Rejected {
                                    reason: format!("Cannot sign the block: {}", e),
                                })
                                .await;
                            Ok(None)
                        })
                        .await
                }
            }
        };

        match signing {
            Ok(maybe_block) => {
                if let Some(block) = maybe_block {
                    let id = block.header.hash();
                    let parent = block.header.block_parent_hash();
                    let chain_length: u32 = block.header.chain_length().into();
                    sender
                        .send(BlockMsg::LeadershipBlock(block))
                        .map_err(|_send_error| LeadershipError::CannotSendLeadershipBlock)
                        .await?;
                    event_logs
                        .set_status(LeadershipLogStatus::Block {
                            block: id.into(),
                            parent: parent.into(),
                            chain_length,
                        })
                        .await;
                };
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    async fn action_schedule(self) -> Result<Self, LeadershipError> {
        let current_slot_position = self.current_slot_position().unwrap();

        let epoch_tip = Epoch(self.tip_ref.block_date().epoch);

        let logger = self.service_info.logger().new(o!(
            "epoch_tip" => epoch_tip.0,
            "current_epoch" => current_slot_position.epoch.0,
            "current_slot" => current_slot_position.slot.0,
        ));

        if epoch_tip < current_slot_position.epoch {
            let (_, leadership, _, _, _, _) = new_epoch_leadership_from(
                current_slot_position.epoch.0,
                Arc::clone(&self.tip_ref),
                false,
            );

            let slot_start = current_slot_position.slot.0 + 1;
            let nb_slots = leadership.era().slots_per_epoch() - slot_start;
            let running_ref = leadership;

            debug!(logger, "scheduling events" ;
                "slot_start" => slot_start,
                "nb_slots" => nb_slots,
            );

            self.action_run_schedule(running_ref, slot_start, nb_slots)
                .await
        } else if epoch_tip == current_slot_position.epoch {
            // check for current epoch
            let slot_start = current_slot_position.slot.0 + 1;
            let nb_slots = self
                .tip_ref
                .epoch_leadership_schedule()
                .era()
                .slots_per_epoch()
                - slot_start;
            let running_ref = Arc::clone(self.tip_ref.epoch_leadership_schedule());

            debug!(logger, "scheduling events" ;
                "slot_start" => slot_start,
                "nb_slots" => nb_slots,
            );

            self.action_run_schedule(running_ref, slot_start, nb_slots)
                .await
        } else {
            // The only reason this would happen is if we had accepted a block
            // that is set in the future or our system local date time is off

            error!(
                logger,
                "It seems the current epoch tip is way ahead of its time."
            );
            Ok(self)
        }
    }

    async fn action_run_schedule(
        self,
        leadership: Arc<Leadership>,
        slot_start: u32,
        nb_slots: u32,
    ) -> Result<Self, LeadershipError> {
        let schedules = self
            .enclave
            .query_schedules(leadership, slot_start, nb_slots)
            .map_err(|e| LeadershipError::CannotScheduleWithEnclave { source: e })
            .await?;

        let mut module = self;
        for schedule in schedules.into_iter() {
            let epoch = Epoch(schedule.date.epoch);
            let slot = EpochSlotOffset(schedule.date.slot_id);
            let scheduled_at_time = module.slot_time(epoch, slot);
            let log = LeadershipLog::new(schedule.id, schedule.date.into(), scheduled_at_time);

            match module.logs.insert(log).await {
                Ok(log) => module.schedule.push(Entry {
                    event: schedule,
                    log,
                }),
                Err(()) => return Err(LeadershipError::CannotUpdateLogs),
            }
        }

        Ok(module)
    }
}

impl Entry {
    fn instant(&self, module: &Module) -> Result<Option<Instant>, LeadershipError> {
        let epoch = Epoch(self.event.date.epoch);
        let slot = EpochSlotOffset(self.event.date.slot_id);
        Ok(module.slot_instant(epoch, slot))
    }
}

impl Schedule {
    pub fn pop(&mut self) -> Option<Entry> {
        self.entries.pop_front()
    }

    pub fn peek(&self) -> Option<&Entry> {
        self.entries.front()
    }

    pub fn push(&mut self, entry: Entry) {
        self.entries.push_back(entry)
    }
}

async fn prepare_block(
    mut fragment_pool: MessageBox<TransactionMsg>,
    block_date: BlockDate,
    ledger: Arc<Ledger>,
    epoch_parameters: Arc<LedgerParameters>,
    logger: Logger,
) -> Result<Contents, LeadershipError> {
    use crate::fragment::selection::FragmentSelectionAlgorithmParams;

    let (reply_handle, reply_future) = unary_reply(logger.clone());

    let msg = TransactionMsg::SelectTransactions {
        ledger: ledger.as_ref().clone(),
        block_date,
        ledger_params: epoch_parameters.as_ref().clone(),
        selection_alg: FragmentSelectionAlgorithmParams::OldestFirst,
        reply_handle,
    };

    if fragment_pool.try_send(msg).is_err() {
        error!(
            logger,
            "cannot send query to the fragment pool for some fragments"
        );
        Err(LeadershipError::CannotConnectToFragmentPool)
    } else {
        reply_future.await.map_err(Into::into)
    }
}

fn too_late(now: SystemTime, event_end: SystemTime) -> bool {
    event_end <= now
}

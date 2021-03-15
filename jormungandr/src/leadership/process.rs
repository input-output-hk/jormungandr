use crate::blockchain::EpochLeadership;
use crate::{
    blockcfg::{
        Block, BlockDate, BlockVersion, Contents, HeaderBuilderNew, LeaderOutput, Leadership,
        Ledger, LedgerParameters,
    },
    blockchain::{new_epoch_leadership_from, Ref, Tip},
    intercom::{unary_reply, BlockMsg, Error as IntercomError, TransactionMsg},
    leadership::{
        enclave::{Enclave, EnclaveError, LeaderEvent, Schedule},
        LeadershipLogHandle, Logs,
    },
    utils::{async_msg::MessageBox, task::TokioServiceInfo},
};
use chain_time::{
    era::{EpochPosition, EpochSlotOffset},
    Epoch, Slot,
};
use futures::{future::TryFutureExt, sink::SinkExt};
use jormungandr_lib::{
    interfaces::{EnclaveLeaderId, LeadershipLog, LeadershipLogStatus},
    time::SystemTime,
};
use std::cmp::Ordering;
use std::{sync::Arc, time::Instant};
use thiserror::Error;
use tracing::{span, Level, Span};
use tracing_futures::Instrument;

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

    #[error("Error while performing a ledger operation")]
    LedgerError(#[from] chain_impl_mockchain::ledger::Error),
}

struct Entry {
    event: LeaderEvent,
    log: LeadershipLogHandle,
}

pub struct Module {
    schedule: Option<Schedule>,
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
            schedule: None,
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

            Err(LeadershipError::TooEarlyForTimeFrame { time: now })
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
        let deadline = self.wait_peek_deadline().await?;
        tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)).await;
        let tip = self.tip.clone();
        self.tip_ref = tip.get_ref().await;
        Ok(self)
    }

    async fn wait_peek_deadline(&mut self) -> Result<Instant, LeadershipError> {
        match self
            .schedule
            .as_mut()
            .expect("schedule must be available at this point")
            .peek()
            .await
        {
            None => {
                // the schedule is empty we were in the _action_ mode, so that means
                // there is no other schedule to have for the current epoch. Better
                // wait for the next epoch

                tracing::debug!("no item scheduled, waiting for next epoch");
                self.next_epoch_instant()
            }
            Some(event) => {
                let span = tracing::span!(
                    parent: self.service_info.span(),
                    Level::TRACE, "leader_event",
                    event_date = %event.date.to_string(),
                    leader_id = %event.id.to_string()
                );

                let epoch = Epoch(event.date.epoch);
                let slot = EpochSlotOffset(event.date.slot_id);
                if let Some(instant) = self.slot_instant(epoch, slot) {
                    async move {
                        tracing::debug!("awaiting");
                        Ok(instant)
                    }
                    .instrument(span)
                    .await
                } else {
                    // if the entry didn't have a valid epoch instant it means
                    // we are looking at passed entry already or it is happening
                    // now. so don't wait any further
                    async move {
                        tracing::debug!("scheduled time for event was missed");
                        Ok(Instant::now())
                    }
                    .instrument(span)
                    .await
                }
            }
        }
    }

    async fn action(mut self) -> Result<Self, LeadershipError> {
        match self.schedule.as_mut() {
            Some(schedule) => match schedule.next().await {
                Some(event) => self.action_entry(event).await,
                None => self.action_schedule().await,
            },
            None => self.action_schedule().await,
        }
    }

    async fn action_entry(self, event: LeaderEvent) -> Result<Self, LeadershipError> {
        let module = self;

        let epoch = Epoch(event.date.epoch);
        let slot = EpochSlotOffset(event.date.slot_id);
        let scheduled_at_time = module.slot_time(epoch, slot);
        let log = LeadershipLog::new(event.id, event.date.into(), scheduled_at_time);

        let entry = match module.logs.insert(log).await {
            Ok(log) => Entry { event, log },
            Err(()) => return Err(LeadershipError::CannotUpdateLogs),
        };

        let end_log = entry.log.clone();
        entry.log.mark_wake().await;
        let module = module.action_run_entry(entry).await?;
        end_log.mark_finished().await;
        Ok(module)
    }

    async fn action_run_entry(self, entry: Entry) -> Result<Self, LeadershipError> {
        let now = SystemTime::now();
        let event_start = self.event_slot_time(&entry.event);
        let event_end = self.event_following_slot_time(&entry.event);

        let span = span!(
            parent: self.service_info.span(),
            Level::TRACE,
            "action_run_entry",
            leader_id = %entry.event.id.to_string(),
            event_date = %entry.event.date.to_string(),
            event_start = %event_start.to_string(),
            event_end = %event_end.to_string()
        );

        async move {
            if too_late(now, event_end) {
                // the event happened out of bounds, ignore it and move to the next one
                tracing::error!(
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
                let right_time = entry.instant(&self);

                if let Some(right_time) = right_time {
                    tracing::warn!(
                        "system woke a bit early for the event, delaying until right time."
                    );

                    // await the right_time before starting the action
                    tokio::time::sleep_until(tokio::time::Instant::from_std(right_time)).await;
                    self.action_run_entry_in_bound(entry, event_end).await
                } else {
                    // because we checked that the entry's slot was below the current
                    // time, if we cannot compute the _right_time_ it means the time
                    // is just starting now to be correct. So it's okay to start
                    // running it now still
                    self.action_run_entry_in_bound(entry, event_end).await
                }
            }
        }
        .instrument(span)
        .await
    }

    async fn action_run_entry_in_bound(
        self,
        entry: Entry,
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
        // handle to the current span, created in `action_run_entry`
        let parent_span = Span::current();
        let span = tracing::span!(
            parent: &parent_span,
            Level::TRACE,
            "action_run_entry_in_bound",
            event_remaining_time = %remaining_time.to_string()
        );
        async move {
            tracing::info!("Leader event started");

        let res = tokio::time::timeout_at(
            tokio::time::Instant::from_std(deadline),
            self.action_run_entry_build_block(entry),
        )
        .await;

            match res {
                Ok(future_res) => future_res,
                Err(timeout_error) => {
                    tracing::error!(reason = %timeout_error, "Eek... took too long to process the event...");
                    event_logs
                        .set_status(LeadershipLogStatus::Rejected {
                            reason: "Failed to compute the schedule within time boundaries".to_owned(),
                        })
                        .await;
                    Ok(())
                }
            }.map(|()| self)
        }.instrument(span).await
    }

    async fn action_run_entry_build_block(&self, entry: Entry) -> Result<(), LeadershipError> {
        let event = entry.event;
        let event_logs = entry.log;

        let enclave = self.enclave.clone();
        let mut sender = self.block_message.clone();
        let pool = self.pool.clone();

        let (parent_id, chain_length) = if self.tip_ref.block_date() < event.date {
            (self.tip_ref.hash(), self.tip_ref.chain_length().increase())
        } else {
            // it appears we are either competing against another stake pool for the same
            // slot or we are a bit behind schedule
            //
            // TODO: check up to a certain distance a valid block to use as parent
            //       for now we will simply exit early
            //
            // * reminder that there is a timeout
            // * jumping epoch is might not be acceptable

            tracing::warn!(
                "It appears the node is running a bit behind schedule, system time might be off?"
            );

            event_logs.set_status(
                    LeadershipLogStatus::Rejected {
                        reason: "Not computing this schedule because of invalid state against the network blockchain".to_owned()
                    }
                ).await;

            return Ok(());
        };

        let current_slot_position = self.current_slot_position().unwrap();
        let EpochLeadership {
            state: ledger,
            ledger_parameters,
            ..
        } = new_epoch_leadership_from(
            current_slot_position.epoch.0,
            Arc::clone(&self.tip_ref),
            false,
        );

        let ledger = ledger.apply_block_step1(chain_length, event.date)?;

        let (contents, ledger) =
            prepare_block(pool, event.id, event.date, ledger, ledger_parameters).await?;

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
                    let ledger = ledger.apply_block_step3(&block.header.to_content_eval_context());
                    sender
                        .send(BlockMsg::LeadershipBlock(block, ledger))
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

        let parent_span = self.service_info.span();
        let span = tracing::span!(
            parent: parent_span,
            Level::TRACE,
            "action_schedule",
            epoch_tip = epoch_tip.0,
            current_epoch = current_slot_position.epoch.0,
            current_slot = current_slot_position.slot.0
        );

        async move {
            match epoch_tip.cmp(&current_slot_position.epoch) {
                Ordering::Less => {
                    let EpochLeadership { leadership, .. } = new_epoch_leadership_from(
                        current_slot_position.epoch.0,
                        Arc::clone(&self.tip_ref),
                        false,
                    );

                    let slot_start = current_slot_position.slot.0 + 1;
                    let nb_slots = leadership.era().slots_per_epoch() - slot_start;
                    let running_ref = leadership;

                    tracing::debug!(
                        slot_start = slot_start,
                        nb_slots = nb_slots,
                        "scheduling events",
                    );

                    self.action_run_schedule(running_ref, slot_start, nb_slots)
                        .await
                }
                Ordering::Equal => {
                    // check for current epoch
                    let slot_start = current_slot_position.slot.0 + 1;
                    let nb_slots = self
                        .tip_ref
                        .epoch_leadership_schedule()
                        .era()
                        .slots_per_epoch()
                        - slot_start;
                    let running_ref = Arc::clone(self.tip_ref.epoch_leadership_schedule());

                    tracing::debug!(
                        slot_start = slot_start,
                        nb_slots = nb_slots,
                        "scheduling events"
                    );

                    self.action_run_schedule(running_ref, slot_start, nb_slots)
                        .await
                }
                Ordering::Greater => {
                    // The only reason this would happen is if we had accepted a block
                    // that is set in the future or our system local date time is off

                    tracing::error!("It seems the current epoch tip is way ahead of its time.");
                    Ok(self)
                }
            }
        }
        .instrument(span)
        .await
    }

    async fn action_run_schedule(
        mut self,
        leadership: Arc<Leadership>,
        slot_start: u32,
        nb_slots: u32,
    ) -> Result<Self, LeadershipError> {
        self.schedule = Some(
            self.enclave
                .query_schedules(leadership, slot_start, nb_slots)
                .map_err(|e| LeadershipError::CannotScheduleWithEnclave { source: e })
                .await?,
        );

        Ok(self)
    }
}

impl Entry {
    fn instant(&self, module: &Module) -> Option<Instant> {
        let epoch = Epoch(self.event.date.epoch);
        let slot = EpochSlotOffset(self.event.date.slot_id);
        module.slot_instant(epoch, slot)
    }
}

async fn prepare_block(
    mut fragment_pool: MessageBox<TransactionMsg>,
    leader_id: EnclaveLeaderId,
    block_date: BlockDate,
    ledger: Ledger,
    epoch_parameters: Arc<LedgerParameters>,
) -> Result<(Contents, Ledger), LeadershipError> {
    use crate::fragment::selection::FragmentSelectionAlgorithmParams;

    let (reply_handle, reply_future) = unary_reply();

    let pool_idx: u32 = leader_id.into();

    let msg = TransactionMsg::SelectTransactions {
        pool_idx: pool_idx as usize,
        ledger,
        block_date,
        ledger_params: epoch_parameters.as_ref().clone(),
        selection_alg: FragmentSelectionAlgorithmParams::OldestFirst,
        reply_handle,
    };

    if fragment_pool.try_send(msg).is_err() {
        tracing::error!("cannot send query to the fragment pool for some fragments");
        Err(LeadershipError::CannotConnectToFragmentPool)
    } else {
        reply_future.await.map_err(Into::into)
    }
}

fn too_late(now: SystemTime, event_end: SystemTime) -> bool {
    event_end <= now
}

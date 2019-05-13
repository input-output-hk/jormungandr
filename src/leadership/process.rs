use crate::{
    blockcfg::{
        Block, BlockBuilder, BlockDate, ChainLength, ConsensusVersion, Epoch, HeaderHash, Leader,
        LeaderOutput,
    },
    blockchain::Tip,
    clock,
    intercom::BlockMsg,
    leadership::{EpochParameters, Leadership, Task, TaskParameters},
    transaction::TPoolR,
    utils::{async_msg::MessageBox, task::TokioServiceInfo},
    BlockchainR,
};
use chain_core::property::{Block as _, BlockDate as _, ChainLength as _};
use slog::Logger;
use std::sync::Arc;
use tokio::{
    prelude::*,
    sync::{mpsc, watch},
};

custom_error! { pub HandleEpochError
    Broadcast = "Cannot broadcast new epoch event to leader tasks, channels closed",
}

custom_error! { pub ProcessError
    EpochHandling { error: HandleEpochError, epoch: Epoch } = "Error while processing new epoch event (epoch: {epoch}): {error}",
    NewEpochReceiver { extra: String } = "Cannot accept anymore epoch events: {extra}",

}

pub struct Process {
    service_info: TokioServiceInfo,

    transaction_pool: TPoolR,
    blockchain_tip: Tip,

    block_message_box: MessageBox<BlockMsg>,

    epoch_broadcaster: watch::Sender<Option<TaskParameters>>,
    epoch_receiver: watch::Receiver<Option<TaskParameters>>,
}

impl Process {
    /// create a new Leadership [`Process`].
    ///
    /// [`Process`]: ./struct.Process.html
    pub fn new(
        service_info: TokioServiceInfo,
        transaction_pool: TPoolR,
        blockchain_tip: Tip,
        block_message_box: MessageBox<BlockMsg>,
    ) -> Self {
        let (epoch_broadcaster, epoch_receiver) = watch::channel(None);

        slog_info!(service_info.logger(), "preparing");

        Process {
            service_info,
            transaction_pool,
            blockchain_tip,
            block_message_box,
            epoch_broadcaster,
            epoch_receiver,
        }
    }

    /// start the `Leadership` process and the associated leader tasks
    pub fn start(
        mut self,
        leaders: Vec<Leader>,
        new_epoch_notifier: mpsc::Receiver<EpochParameters>,
    ) -> impl Future<Item = (), Error = ()> {
        let error_logger = self.service_info.logger().clone();
        slog_info!(self.service_info.logger(), "starting");

        for leader in leaders {
            self.spawn_leader(leader);
        }

        new_epoch_notifier
            .map_err(|err| ProcessError::NewEpochReceiver {
                extra: format!("{}", err),
            })
            .for_each(move |epoch_parameters| {
                let epoch = epoch_parameters.epoch;
                if let Err(error) = self.handle_epoch(epoch_parameters) {
                    futures::future::err(ProcessError::EpochHandling {
                        error: error,
                        epoch: epoch,
                    })
                } else {
                    futures::future::ok(())
                }
            })
            .map_err(move |error| {
                slog_crit!(error_logger, "Error in the Leadership Process" ; "reason" => error.to_string())
            })
    }

    /// spawn a new leader [`Task`] in the `Process` runtime.
    ///
    /// [`Task`]: ./struct.Task.html
    fn spawn_leader(&mut self, leader: Leader) {
        let epoch_receiver = self.epoch_receiver.clone();
        let blockchain_tip = self.blockchain_tip.clone();
        let logger = self.service_info.logger().clone();
        let transaction_pool = self.transaction_pool.clone();
        let block_message = self.block_message_box.clone();
        let task = Task::new(
            logger,
            leader,
            blockchain_tip,
            transaction_pool,
            epoch_receiver,
            block_message,
        );

        self.service_info.spawn(task.start())
    }

    /// handle incoming Epoch
    fn handle_epoch(&mut self, epoch_parameters: EpochParameters) -> Result<(), HandleEpochError> {
        let leadership =
            Leadership::new(epoch_parameters.epoch, &epoch_parameters.ledger_reference);

        let task_parameters = TaskParameters {
            epoch: epoch_parameters.epoch,
            ledger_static_parameters: epoch_parameters.ledger_static_parameters,
            ledger_parameters: epoch_parameters.ledger_parameters,
            leadership: Arc::new(leadership),
            time_frame: epoch_parameters.time_frame,
        };

        self.epoch_broadcaster
            .broadcast(Some(task_parameters))
            .map_err(|_| HandleEpochError::Broadcast)
    }
}

pub fn leadership_task(
    service_info: TokioServiceInfo,
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
    let consensus = state.consensus_version();
    let parameters = state.get_ledger_parameters();

    let leadership = match consensus {
        ConsensusVersion::Bft => b
            .get_leadership_or_build(date.epoch, &last_block.id())
            .unwrap(),
        ConsensusVersion::GenesisPraos => b
            .get_leadership(date.epoch - 2)
            .or_else(|| b.get_leadership(date.epoch - 1))
            .or_else(|| b.get_leadership(date.epoch))
            .or_else(|| b.get_leadership_or_build(date.epoch, &last_block.id()))
            .unwrap(),
    };

    let parent_id = b.get_tip().unwrap();

    let logger = Logger::root(
        logger.clone(),
        o!(
            "chain_length" => chain_length.to_string(),
            "consensus-version" => consensus.to_string(),
            "allow_accounts" => parameters.allow_account_creation,
        ),
    );

    // let am_leader = leadership.get_leader_at(date.clone()).unwrap() == leader_id;
    match leadership.is_leader_for_date(&secret, date).unwrap() {
        LeaderOutput::None => None,
        LeaderOutput::Bft(_bft_public_key) => {
            if let Some(bft_secret_key) = &secret.bft_leader {
                slog_info!(logger, "Node elected for BFT");
                let block_builder = prepare_block(&transaction_pool, date, chain_length, parent_id);

                let block = block_builder.make_bft_block(&bft_secret_key.sig_key);

                assert!(leadership.verify(&block.header).success());
                Some(block)
            } else {
                slog_crit!(
                    logger,
                    "Node was elected for BFT, but does not have the setting"
                );
                None
            }
        }
        LeaderOutput::GenesisPraos(witness) => {
            if let Some(genesis_leader) = &mut secret.genesis_leader {
                slog_info!(logger, "Node elected for Genesis Praos");
                let block_builder = prepare_block(&transaction_pool, date, chain_length, parent_id);

                let block = block_builder.make_genesis_praos_block(
                    &genesis_leader.node_id,
                    &mut genesis_leader.sig_key,
                    witness,
                );

                assert!(leadership.verify(&block.header).success());
                Some(block)
            } else {
                slog_crit!(
                    logger,
                    "Node was elected for Genesis Praos, but does not have the setting"
                );
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

use crate::{
    fragment::{Logs, Pool},
    intercom::TransactionMsg,
    stats_counter::StatsCounter,
    utils::{async_msg::MessageQueue, task::TokioServiceInfo},
};
use slog::Logger;
use std::time::Duration;
use tokio::{
    prelude::{
        future::Either::{A, B},
        *,
    },
    timer::Interval,
};

pub struct Process {
    pool: Pool,
    logs: Logs,
    garbage_collection_interval: Duration,
}

impl Process {
    pub fn new(
        pool_ttl: Duration,
        logs_ttl: Duration,
        garbage_collection_interval: Duration,
    ) -> Self {
        let logs = Logs::new(logs_ttl);
        Process {
            pool: Pool::new(pool_ttl, logs.clone()),
            logs,
            garbage_collection_interval,
        }
    }

    pub fn logs(&self) -> &Logs {
        &self.logs
    }
    pub fn pool(&self) -> &Pool {
        &self.pool
    }

    pub fn start(
        self,
        service_info: TokioServiceInfo,
        stats_counter: StatsCounter,
        input: MessageQueue<TransactionMsg>,
    ) -> impl Future<Item = (), Error = ()> {
        service_info.spawn(self.start_pool_garbage_collector(service_info.logger().clone()));
        input.for_each(move |input| {
            match input {
                TransactionMsg::ProposeTransaction(txids, reply) => {
                    let logs = self.pool.logs().clone();

                    A(A(logs.exists(txids).and_then(|rep| {
                        reply.reply_ok(rep);
                        future::ok(())
                    })))
                }
                TransactionMsg::SendTransaction(origin, txs) => {
                    // Note that we cannot use apply_block here, since we don't have a valid context to which to apply
                    // those blocks. one valid tx in a given context, could be invalid in another. for example
                    // fee calculations, existence utxo / account solvency.

                    // FIXME/TODO check that the txs are valid within themselves with basic requirements (e.g. inputs >= outputs).
                    // we also want to keep a basic capability to filter away repetitive queries or definitely discarded txid.

                    // This interface only makes sense for messages coming from arbitrary users (like transaction, certificates),
                    // for other message we don't want to receive them through this interface, and possibly
                    // put them in another pool.

                    let stats_counter = stats_counter.clone();
                    A(B(self
                        .pool
                        .clone()
                        .insert_all(origin, txs)
                        .map(move |count| stats_counter.add_tx_recv_cnt(count))))
                }
                TransactionMsg::GetTransactions(_txids, _handler) => {
                    // this function is no yet implemented, this is not handled in the
                    B(future::ok(unimplemented!()))
                }
            }
        })
    }

    fn start_pool_garbage_collector(&self, logger: Logger) -> impl Future<Item = (), Error = ()> {
        let mut pool = self.pool().clone();
        let garbage_collection_interval = self.garbage_collection_interval;
        let error_logger = logger.clone();
        Interval::new_interval(garbage_collection_interval)
            .for_each(move |_instant| {
                debug!(logger, "garbage collect entries in the MemPool and in the logs");
                pool.poll_purge()
            })
            .map_err(move |error| {
                error!(error_logger, "Cannot run the MemPool garbage collection" ; "reason" => error.to_string());
            })
    }
}

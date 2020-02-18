use crate::{
    fragment::{Logs, Pool},
    intercom::{NetworkMsg, TransactionMsg},
    stats_counter::StatsCounter,
    utils::{
        async_msg::{channel, MessageBox, MessageQueue},
        task::TokioServiceInfo,
    },
};
use futures03::{compat::*, sink::SinkExt};
use std::time::Duration;
use tokio02::stream::StreamExt;

pub struct Process {
    pool: Pool,
    garbage_collection_interval: Duration,
}

impl Process {
    pub fn new(
        pool_max_entries: usize,
        pool_ttl: Duration,
        logs_max_entries: usize,
        logs_ttl: Duration,
        garbage_collection_interval: Duration,
        network_msg_box: MessageBox<NetworkMsg>,
    ) -> Self {
        let logs = Logs::new(logs_max_entries, logs_ttl);
        Process {
            pool: Pool::new(pool_max_entries, pool_ttl, logs, network_msg_box),
            garbage_collection_interval,
        }
    }

    pub async fn start(
        self,
        service_info: TokioServiceInfo,
        stats_counter: StatsCounter,
        input: MessageQueue<TransactionMsg>,
    ) -> Result<(), ()> {
        let (gc_sender, gc_receiver) = channel(1);

        service_info.run_periodic_std(
            "pool garbage collection",
            self.garbage_collection_interval,
            move || {
                let gc_sender = gc_sender.clone();
                async move {
                    gc_sender
                        .sink_compat()
                        .send(TransactionMsg::RunGarbageCollector)
                        .await
                }
            },
        );

        let mut input = input.compat().merge(gc_receiver.compat());
        let mut pool = self.pool;

        while let Some(input_result) = input.next().await {
            match input_result? {
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

                    pool.insert_and_propagate_all(origin, txs, service_info.logger().clone())
                        .await
                        .map(move |count| stats_counter.add_tx_recv_cnt(count))?;
                }
                TransactionMsg::RemoveTransactions(fragment_ids, status) => {
                    pool.remove_added_to_block(fragment_ids, status);
                }
                TransactionMsg::GetLogs(reply_handle) => {
                    let logs = pool.logs().logs();
                    reply_handle.reply_ok(logs);
                }
                TransactionMsg::SelectTransactions {
                    ledger,
                    block_date,
                    ledger_params,
                    selection_alg,
                    reply_handle,
                } => {
                    let contents = pool.select(ledger, block_date, ledger_params, selection_alg);
                    reply_handle.reply_ok(contents);
                }
                TransactionMsg::RunGarbageCollector => {
                    let _ = pool.poll_purge().await;
                }
            }
        }

        Ok(())
    }
}

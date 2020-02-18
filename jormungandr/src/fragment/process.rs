use crate::{
    fragment::{Logs, Pool},
    intercom::{NetworkMsg, TransactionMsg},
    stats_counter::StatsCounter,
    utils::{
        async_msg::{MessageBox, MessageQueue},
        task::TokioServiceInfo,
    },
};
use futures03::{compat::*, stream::StreamExt};
use std::time::Duration;

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
        self.start_pool_garbage_collector(&service_info);
        let mut input = input.compat();
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

                    self.pool
                        .clone()
                        .insert_and_propagate_all(origin, txs, service_info.logger().clone())
                        .await
                        .map(move |count| stats_counter.add_tx_recv_cnt(count))?;
                }
                TransactionMsg::RemoveTransactions(fragment_ids, status) => {
                    self.pool
                        .clone()
                        .remove_added_to_block(fragment_ids, status)
                        .await;
                }
                TransactionMsg::GetLogs(reply_handle) => {
                    let logs = self.pool.logs().logs().await;
                    reply_handle.reply_ok(logs);
                }
                TransactionMsg::SelectTransactions {
                    ledger,
                    block_date,
                    ledger_params,
                    selection_alg,
                    reply_handle,
                } => {
                    let contents = self
                        .pool
                        .clone()
                        .select(ledger, block_date, ledger_params, selection_alg)
                        .await;
                    reply_handle.reply_ok(contents);
                }
            }
        }

        Ok(())
    }

    fn start_pool_garbage_collector(&self, service_info: &TokioServiceInfo) {
        let pool = self.pool.clone();
        service_info.run_periodic_std(
            "pool garbage collection",
            self.garbage_collection_interval,
            move || {
                let mut pool = pool.clone();
                async move { pool.poll_purge().await }
            },
        )
    }
}

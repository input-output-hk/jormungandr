use crate::{
    fragment::{Logs, Pool},
    intercom::{NetworkMsg, TransactionMsg},
    stats_counter::StatsCounter,
    utils::{
        async_msg::{MessageBox, MessageQueue},
        task::TokioServiceInfo,
    },
};
use tokio02::stream::StreamExt;

pub struct Process {
    pool: Pool,
}

impl Process {
    pub fn new(
        pool_max_entries: usize,
        logs_max_entries: usize,
        network_msg_box: MessageBox<NetworkMsg>,
    ) -> Self {
        let logs = Logs::new(logs_max_entries);
        Process {
            pool: Pool::new(pool_max_entries, logs, network_msg_box),
        }
    }

    pub async fn start(
        self,
        service_info: TokioServiceInfo,
        stats_counter: StatsCounter,
        mut input: MessageQueue<TransactionMsg>,
    ) -> Result<(), ()> {
        let mut pool = self.pool;

        while let Some(input_result) = input.next().await {
            match input_result {
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
                    let logs = pool.logs().logs().cloned().collect();
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
            }
        }

        Ok(())
    }
}

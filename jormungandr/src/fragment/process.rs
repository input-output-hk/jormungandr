use crate::{
    fragment::{Logs, Pools},
    intercom::{NetworkMsg, TransactionMsg},
    stats_counter::StatsCounter,
    utils::{
        async_msg::{MessageBox, MessageQueue},
        task::TokioServiceInfo,
    },
};

use std::collections::HashMap;

use thiserror::Error;
use tokio_stream::StreamExt;
use tracing::{span, Level};
use tracing_futures::Instrument;

pub struct Process {
    pool_max_entries: usize,
    logs: Logs,
    network_msg_box: MessageBox<NetworkMsg>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("transaction pool error")]
    Pool(#[from] crate::fragment::pool::Error),
}

impl Process {
    pub fn new(
        pool_max_entries: usize,
        logs_max_entries: usize,
        network_msg_box: MessageBox<NetworkMsg>,
    ) -> Self {
        let logs = Logs::new(logs_max_entries);
        Process {
            pool_max_entries,
            logs,
            network_msg_box,
        }
    }

    pub async fn start(
        self,
        n_pools: usize,
        service_info: TokioServiceInfo,
        stats_counter: StatsCounter,
        mut input: MessageQueue<TransactionMsg>,
    ) -> Result<(), Error> {
        let mut pool = Pools::new(
            self.pool_max_entries,
            n_pools,
            self.logs,
            self.network_msg_box,
        );

        async move {
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

                        pool.insert_and_propagate_all(origin, txs)
                            .await
                            .map(move |count| stats_counter.add_tx_recv_cnt(count))?;
                    }
                    TransactionMsg::RemoveTransactions(fragment_ids, status) => {
                        tracing::debug!(
                            "removing fragments added to block {:?}: {:?}",
                            status,
                            fragment_ids
                        );
                        pool.remove_added_to_block(fragment_ids, status);
                    }
                    TransactionMsg::GetLogs(reply_handle) => {
                        let logs = pool.logs().logs().cloned().collect();
                        reply_handle.reply_ok(logs);
                    }
                    TransactionMsg::GetStatuses(fragment_ids, reply_handle) => {
                        let mut statuses = HashMap::new();
                        pool.logs().logs_by_ids(fragment_ids).into_iter().for_each(
                            |(fragment_id, log)| {
                                statuses.insert(fragment_id, log.status().clone());
                            },
                        );
                        reply_handle.reply_ok(statuses);
                    }
                    TransactionMsg::SelectTransactions {
                        pool_idx,
                        ledger,
                        ledger_params,
                        selection_alg,
                        reply_handle,
                        soft_deadline_future,
                        hard_deadline_future,
                    } => {
                        let contents = pool
                            .select(
                                pool_idx,
                                ledger,
                                ledger_params,
                                selection_alg,
                                soft_deadline_future,
                                hard_deadline_future,
                            )
                            .await;
                        reply_handle.reply_ok(contents);
                    }
                }
            }
            Ok(())
        }
        .instrument(span!(parent: service_info.span(), Level::TRACE, "process", kind = "fragment"))
        .await
    }
}

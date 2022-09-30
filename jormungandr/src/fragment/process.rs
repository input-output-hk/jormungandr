use crate::{
    blockchain::Tip,
    fragment::{Logs, Pool},
    intercom::{NetworkMsg, TransactionMsg},
    metrics::{Metrics, MetricsBackend},
    utils::{
        async_msg::{MessageBox, MessageQueue},
        task::TokioServiceInfo,
    },
};
use futures::{future, TryFutureExt};
use std::{
    collections::HashMap,
    convert::TryInto,
    io,
    path::{Path, PathBuf},
};
use thiserror::Error;
use time::{macros::format_description, Duration, OffsetDateTime, Time};
use tokio::fs::{self, File};
use tokio_stream::StreamExt;
use tracing::{debug_span, span, Level};
use tracing_futures::Instrument;

pub struct Process {
    pool_max_entries: usize,
    logs_max_entries: usize,
    network_msg_box: MessageBox<NetworkMsg>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("transaction pool error")]
    Pool(#[from] crate::fragment::pool::Error),
    #[error("failed to open persistent log file")]
    PersistentLog(#[source] io::Error),
}

impl Process {
    pub fn new(
        pool_max_entries: usize,
        logs_max_entries: usize,
        network_msg_box: MessageBox<NetworkMsg>,
    ) -> Self {
        Process {
            pool_max_entries,
            logs_max_entries,
            network_msg_box,
        }
    }

    pub async fn start<P: AsRef<Path>>(
        self,
        service_info: TokioServiceInfo,
        stats_counter: Metrics,
        mut input: MessageQueue<TransactionMsg>,
        persistent_log_dir: Option<P>,
        tip: Tip,
    ) -> Result<(), Error> {
        async fn hourly_wakeup(enabled: bool) {
            if enabled {
                let now = OffsetDateTime::now_utc();
                // truncate date to hour so that rotation always happens at the hour mark
                let current_hour = now.replace_time(Time::from_hms(now.hour(), 0, 0).unwrap());
                let next_hour = current_hour + Duration::HOUR;
                tokio::time::sleep((next_hour - now).try_into().unwrap()).await
            } else {
                future::pending().await
            }
        }

        async fn open_log_file(dir: &Path) -> Result<File, Error> {
            let mut path: PathBuf = dir.into();
            if !path.exists() {
                std::fs::create_dir_all(dir).map_err(Error::PersistentLog)?;
            }
            let log_file_name = OffsetDateTime::now_utc()
                .format(format_description!("[year]-[month]-[day]_[hour].log"))
                .expect("invalid time format description");
            path.push(log_file_name);
            tracing::debug!("creating fragment log file `{:?}`", path);
            fs::OpenOptions::new()
                .append(true)
                .create(true)
                .read(false)
                .open(path)
                .map_err(Error::PersistentLog)
                .await
        }

        if self.logs_max_entries < self.pool_max_entries {
            tracing::warn!(
                "Having 'log_max_entries' < 'pool_max_entries' is not recommendend. Overriding 'log_max_entries' to {}", self.pool_max_entries
            );
        }
        let logs = Logs::new(std::cmp::max(self.logs_max_entries, self.pool_max_entries));

        let mut wakeup = Box::pin(hourly_wakeup(persistent_log_dir.is_some()));

        async move {
            let persistent_log = match &persistent_log_dir {
                None => None,
                Some(dir) => {
                    let file = open_log_file(dir.as_ref()).await?;
                    Some(file)
                }
            };

            let mut pool = Pool::new(
                self.pool_max_entries,
                logs,
                self.network_msg_box,
                persistent_log,
                tip,
                stats_counter.clone()
            );

            loop {
                tokio::select! {
                    maybe_msg = input.next() => {
                        tracing::trace!("handling new fragment task item");
                        match maybe_msg {
                            None => break,
                            Some(msg) => match msg {
                                TransactionMsg::SendTransactions { origin, fragments, fail_fast, reply_handle } => {
                                    // Note that we cannot use apply_block here, since we don't have a valid context to which to apply
                                    // those blocks. one valid tx in a given context, could be invalid in another. for example
                                    // fee calculations, existence utxo / account solvency.

                                    // FIXME/TODO check that the txs are valid within themselves with basic requirements (e.g. inputs >= outputs).
                                    // we also want to keep a basic capability to filter away repetitive queries or definitely discarded txid.

                                    // This interface only makes sense for messages coming from arbitrary users (like transaction, certificates),
                                    // for other message we don't want to receive them through this interface, and possibly
                                    // put them in another pool.
                                    let span = debug_span!("incoming_fragments");
                                    async {
                                        let stats_counter = stats_counter.clone();
                                        let summary = pool
                                            .insert_and_propagate_all(origin, fragments, fail_fast)
                                            .await?;

                                        stats_counter.add_tx_recv_cnt(summary.accepted.len());

                                        reply_handle.reply_ok(summary);
                                        Ok::<(), Error>(())
                                    }
                                    .instrument(span)
                                    .await?;
                                }
                                TransactionMsg::RemoveTransactions(fragment_ids, status) => {
                                    let span = debug_span!("remove_transactions_in_block");
                                    async {
                                        tracing::debug!(
                                            "removing fragments added to block {:?}: {:?}",
                                            status,
                                            fragment_ids
                                        );
                                        pool.remove_added_to_block(fragment_ids, status);
                                        pool.remove_expired_txs().await;
                                    }.instrument(span).await
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
                                TransactionMsg::BranchSwitch(fork_date) => {
                                    tracing::debug!(%fork_date, "pruning logs after branch switch");
                                    pool.prune_after_ledger_branch(fork_date);
                                }
                                TransactionMsg::SelectTransactions {
                                    ledger,
                                    selection_alg,
                                    reply_handle,
                                    soft_deadline_future,
                                    hard_deadline_future,
                                } => {
                                    let span = span!(
                                        Level::DEBUG,
                                        "fragment_selection",
                                        kind = "older_first",
                                    );
                                    async {
                                        let contents = pool
                                        .select(
                                            ledger,
                                            selection_alg,
                                            soft_deadline_future,
                                            hard_deadline_future,
                                        )
                                        .await;
                                        reply_handle.reply_ok(contents);
                                    }
                                    .instrument(span)
                                    .await
                                }
                            }
                        };
                        tracing::trace!("item handling finished");
                    }
                    _ = &mut wakeup => {
                        async {
                            pool.close_persistent_log().await;
                            let dir = persistent_log_dir.as_ref().unwrap();
                            let file = open_log_file(dir.as_ref()).await?;
                            pool.set_persistent_log(file);
                            wakeup = Box::pin(hourly_wakeup(true));
                            Ok::<_, Error>(())
                        }
                        .instrument(debug_span!("persistent_log_rotation")).await?;
                    }
                }
            }
            Ok(())
        }
        .instrument(span!(parent: service_info.span(), Level::TRACE, "process", kind = "fragment"))
        .await
    }
}

use super::{Gossip, Gossips, P2pTopology, Peer};
use crate::{
    intercom::{NetworkMsg, PropagateMsg, TopologyMsg},
    metrics::Metrics,
    settings::start::network::Configuration,
    utils::async_msg::{MessageBox, MessageQueue},
};
use std::time::Duration;
use tokio::time::{Instant, Interval, MissedTickBehavior};
use tokio_stream::StreamExt;

pub const DEFAULT_NETWORK_STUCK_INTERVAL: Duration = Duration::from_secs(60 * 5); // 5 min
const QUARANTINE_CHECK: Duration = Duration::from_secs(60);
const MAX_GOSSIP_SIZE: usize = 10;

struct Process {
    input: MessageQueue<TopologyMsg>,
    network_msgbox: MessageBox<NetworkMsg>,
    gossip_interval: Interval,
    network_stuck_check: Duration,
    topology: P2pTopology,
}

pub struct TaskData {
    pub network_msgbox: MessageBox<NetworkMsg>,
    pub topology_queue: MessageQueue<TopologyMsg>,
    pub initial_peers: Vec<Peer>,
    pub config: Configuration,
    pub stats_counter: Metrics,
}

pub async fn start(task_data: TaskData) {
    let TaskData {
        network_msgbox,
        topology_queue,
        initial_peers,
        config,
        stats_counter,
    } = task_data;

    let mut topology = P2pTopology::new(&config, stats_counter);

    topology.accept_gossips(Gossips::from(
        initial_peers
            .into_iter()
            .map(Gossip::from)
            .collect::<Vec<_>>(),
    ));

    let mut gossip_interval = tokio::time::interval(config.gossip_interval);
    gossip_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    let mut process = Process {
        input: topology_queue,
        gossip_interval,
        network_stuck_check: config.network_stuck_check,
        network_msgbox,
        topology,
    };
    process.handle_input().await;
}

impl Process {
    async fn handle_input(&mut self) {
        let mut last_update = Instant::now();
        let mut quarantine_check = tokio::time::interval(QUARANTINE_CHECK);

        loop {
            tokio::select! {
                Some(input) = self.input.next() => {
                    tracing::trace!("handling new topology task item");
                    match input {
                        TopologyMsg::AcceptGossip(gossip) => {
                            self.topology.accept_gossips(gossip);
                            last_update = Instant::now();
                        },
                        TopologyMsg::DemotePeer(id) => self.topology.report_node(&id),
                        TopologyMsg::PromotePeer(id) => self.topology.promote_node(&id),
                        TopologyMsg::View(selection, handle) => {
                            handle.reply_ok(self.topology.view(selection))
                        }
                        TopologyMsg::ListAvailable(handle) => {
                            handle.reply_ok(self.topology.list_available().map(Into::into).collect::<Vec<_>>())
                        }
                        TopologyMsg::ListNonPublic(handle) => {
                            handle.reply_ok(self.topology.list_non_public().map(Into::into).collect::<Vec<_>>())
                        }
                        TopologyMsg::ListQuarantined(handle) => {
                            handle.reply_ok(self.topology.list_quarantined())
                        }
                    }
                    tracing::trace!("item handling finished");
                },
                _ = self.gossip_interval.tick() => {
                        let span = tracing::debug_span!("generating_gossip", task = "topology");
                        let _guard = span.enter();
                        self.topology.update_gossip();
                        let view = self.topology.view(poldercast::layer::Selection::Any);
                        if view.peers.is_empty() {
                            tracing::warn!("no peers to gossip with found, check your connection");
                        }
                        tracing::trace!("gossiping with peers");
                        self.send_gossip_messages(view.peers)
                    }
                _ = quarantine_check.tick() => {
                    let span = tracing::debug_span!("quarantine_check", task = "topology");
                    let _guard = span.enter();
                    // Even if lifted from quarantine, peers will be re-added to the topology
                    // only after we receive a gossip about them.
                    let mut nodes_to_contact = self.topology.lift_reports();

                    // If we did not receive any incoming gossip recently let's try to contact known (but not active) nodes.
                    if last_update.elapsed() > self.network_stuck_check {
                        last_update = Instant::now();
                        tracing::warn!("p2p network have been too quiet for some time, will try to contanct known nodes");
                        nodes_to_contact.extend(
                            self.topology
                                .list_available()
                                .take(MAX_GOSSIP_SIZE.saturating_sub(nodes_to_contact.len()))
                        );
                    }

                    self.send_gossip_messages(nodes_to_contact);
                }
            }
        }
    }

    fn send_gossip_messages(&mut self, peers: Vec<Peer>) {
        for peer in peers {
            let gossip = self.topology.initiate_gossips(&peer.id());
            self.network_msgbox
                // do not block the current thread to avoid deadlocks
                .try_send(NetworkMsg::Propagate(Box::new(PropagateMsg::Gossip(
                    peer, gossip,
                ))))
                .unwrap_or_else(|e| {
                    tracing::error!(
                        reason = ?e,
                        "cannot send PropagateMsg request to network"
                    )
                });
        }
    }
}

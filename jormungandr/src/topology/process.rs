use super::{Gossip, Gossips, P2pTopology, Peer};
use crate::intercom::{NetworkMsg, PropagateMsg, TopologyMsg};
use crate::settings::start::network::Configuration;
use crate::utils::async_msg::{MessageBox, MessageQueue};
use std::time::Duration;
use tokio::time::{Instant, Interval};
use tokio_stream::StreamExt;

pub const DEFAULT_NETWORK_STUCK_INTERVAL: Duration = Duration::from_secs(60 * 5); // 5 min

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
}

pub async fn start(task_data: TaskData) {
    let TaskData {
        network_msgbox,
        topology_queue,
        initial_peers,
        config,
    } = task_data;

    let mut topology = P2pTopology::new(&config);

    topology.accept_gossips(Gossips::from(
        initial_peers
            .into_iter()
            .map(Gossip::from)
            .collect::<Vec<_>>(),
    ));

    let mut process = Process {
        input: topology_queue,
        gossip_interval: tokio::time::interval(config.gossip_interval),
        network_stuck_check: config.network_stuck_check,
        network_msgbox,
        topology,
    };
    process.handle_input().await;
}

impl Process {
    async fn handle_input(&mut self) {
        let mut last_update = Instant::now();
        let mut stuck_inverval = tokio::time::interval(Duration::from_secs(60));

        loop {
            tokio::select! {
                Some(input) = self.input.next() => {
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
                            handle.reply_ok(self.topology.list_available())
                        }
                        TopologyMsg::ListNonPublic(handle) => {
                            handle.reply_ok(self.topology.list_non_public())
                        }
                        TopologyMsg::ListQuarantined(handle) => {
                            handle.reply_ok(self.topology.list_quarantined())
                        }
                    }
                },
                _ = self.gossip_interval.tick() => {
                        self.topology.update_gossip();
                        let view = self.topology.view(poldercast::layer::Selection::Any);
                        if view.peers.is_empty() {
                            tracing::warn!("no peers to gossip with found, check your connection");
                        }
                        self.send_gossip_messages(view.peers)
                    }
                // For some reason, resetting a tokio::time::Sleep instance when receiving a new gossip
                // result in a stack overflow / allocation failure in some tests.
                // This should achieve the same thing, without hopefully the blowing-up-the-stack part
                _ = stuck_inverval.tick() => {
                    if last_update.elapsed() >= self.network_stuck_check {
                        tracing::warn!("p2p network have been too quiet for some time, will try to contact nodes for which quarantine have elapsed");
                        let quarantined_nodes = self.topology.lift_nodes_from_quarantine();
                        self.send_gossip_messages(quarantined_nodes);

                        last_update = Instant::now();
                    }
                }
            }
        }
    }

    fn send_gossip_messages(&mut self, peers: Vec<Peer>) {
        for peer in peers {
            let gossip = self.topology.initiate_gossips(&peer.id());
            self.network_msgbox
                // do not block the current thread to avoid deadlocks
                .try_send(NetworkMsg::Propagate(PropagateMsg::Gossip(peer, gossip)))
                .unwrap_or_else(|e| {
                    tracing::error!(
                        reason = ?e,
                        "cannot send PropagateMsg request to network"
                    )
                });
        }
    }
}

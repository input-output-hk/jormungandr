use super::{P2pTopology, Peer};
use crate::intercom::{NetworkMsg, PropagateMsg, TopologyMsg};
use crate::settings::start::network::Configuration;
use crate::utils::async_msg::{MessageBox, MessageQueue};
use futures::SinkExt;
use tokio::time::Interval;
use tokio_stream::StreamExt;

struct Process {
    input: MessageQueue<TopologyMsg>,
    network_msgbox: MessageBox<NetworkMsg>,
    gossip_interval: Interval,
    topology: P2pTopology,
}

pub struct TaskData {
    pub network_msgbox: MessageBox<NetworkMsg>,
    pub topology_queue: MessageQueue<TopologyMsg>,
    pub config: Configuration,
}

pub async fn start(task_data: TaskData) {
    let TaskData {
        mut network_msgbox,
        topology_queue,
        config,
    } = task_data;

    let mut topology = P2pTopology::new(&config);

    // Send gossips for trusted peers at the beginning, without inserting them first in
    // the topology
    for peer in config.trusted_peers {
        let gossips = topology.initiate_gossips(None);
        network_msgbox
            .send(NetworkMsg::Propagate(PropagateMsg::Gossip(
                Peer {
                    addr: peer.addr,
                    id: peer.id,
                },
                gossips,
            )))
            .await
            .unwrap_or_else(|e| tracing::error!("Error sending gossips to network task: {}", e));
    }

    let mut process = Process {
        input: topology_queue,
        gossip_interval: tokio::time::interval(config.gossip_interval),
        network_msgbox,
        topology,
    };
    process.handle_input().await;
}

impl Process {
    async fn handle_input(&mut self) {
        loop {
            tokio::select! {
                Some(input) = self.input.next() => {
                    match input {
                        TopologyMsg::AcceptGossip(gossip) => self.topology.accept_gossips(gossip),
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
                        let view = self.topology.view(poldercast::layer::Selection::Any);
                        for peer in view.peers {
                            // Peers returned by the topology will always have a NodeId
                            let id = peer.id.clone().unwrap();
                            let gossip = self.topology.initiate_gossips(Some(&id));

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
        }
    }
}

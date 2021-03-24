use super::{Gossips, P2pTopology, Peer};
use crate::intercom::{self, NetworkMsg, PropagateMsg, ReplyHandle, TopologyMsg};
use crate::settings::start::network::Configuration;
use crate::utils::async_msg::{self, MessageBox, MessageQueue};
use crate::utils::task::TokioServiceInfo;
use futures::stream::Stream;
use futures::SinkExt;
use tokio_stream::StreamExt;

const INTERNAL_QUEUE_LEN: usize = 32;

struct Process<T: Stream<Item = Message> + Unpin> {
    input: T,
    topology: P2pTopology,
}

pub struct TaskData {
    pub network_msgbox: MessageBox<NetworkMsg>,
    pub topology_queue: MessageQueue<TopologyMsg>,
    pub config: Configuration,
}

enum Message {
    // This is the public interface of P2PTopology
    Public(TopologyMsg),
    // This is the task private interface of P2PTopology
    Private(PrivateMsg),
}

// Interaction with the topology that is used internally
// and is not available to other tasks
enum PrivateMsg {
    GetGossips(ReplyHandle<Vec<(Peer, Gossips)>>),
    ResetTopology,
}

pub async fn start(info: TokioServiceInfo, task_data: TaskData) {
    let TaskData {
        mut network_msgbox,
        topology_queue,
        config,
    } = task_data;

    let mut topology = P2pTopology::new(&config);

    let public_input = topology_queue.map(Message::Public);
    let (internal_msgbox, internal_queue) = async_msg::channel(INTERNAL_QUEUE_LEN);
    let input = public_input.merge(internal_queue.map(Message::Private));

    let internal_msgbox_1 = internal_msgbox.clone();
    let internal_msgbox_2 = internal_msgbox.clone();

    if let Some(interval) = config.topology_force_reset_interval {
        info.run_periodic_fallible("topology layers force reset", interval, move || {
            reset_layers(internal_msgbox_1.clone())
        });
    }

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

    info.run_periodic_fallible(
        "send gossips to network task to propagate",
        config.gossip_interval,
        move || handle_gossip(internal_msgbox_2.clone(), network_msgbox.clone()),
    );

    let mut process = Process { input, topology };

    process.handle_input().await;
}

impl<T: Stream<Item = Message> + Unpin> Process<T> {
    async fn handle_input(&mut self) {
        use Message::{Private, Public};
        while let Some(input) = self.input.next().await {
            match input {
                Public(TopologyMsg::AcceptGossip(gossip)) => self.topology.accept_gossips(gossip),
                Public(TopologyMsg::DemotePeer(id)) => self.topology.report_node(&id),
                Public(TopologyMsg::PromotePeer(id)) => self.topology.promote_node(&id),
                Public(TopologyMsg::View(selection, handle)) => {
                    handle.reply_ok(self.topology.view(selection))
                }
                Public(TopologyMsg::ListAvailable(handle)) => {
                    handle.reply_ok(self.topology.list_available())
                }
                Public(TopologyMsg::ListNonPublic(handle)) => {
                    handle.reply_ok(self.topology.list_non_public())
                }
                Public(TopologyMsg::ListQuarantined(handle)) => {
                    handle.reply_ok(self.topology.list_quarantined())
                }
                Private(PrivateMsg::ResetTopology) => self.topology.force_reset_layers(),
                Private(PrivateMsg::GetGossips(handle)) => {
                    let view = self.topology.view(poldercast::layer::Selection::Any);
                    let mut res = Vec::new();
                    for peer in view.peers {
                        // Peers returned bu the topology will always have a NodeId
                        let id = peer.id.clone().unwrap();
                        res.push((peer, self.topology.initiate_gossips(Some(&id))));
                    }
                    handle.reply_ok(res);
                }
            }
        }
    }
}

async fn reset_layers(
    mut mbox: MessageBox<PrivateMsg>,
) -> Result<(), futures::channel::mpsc::SendError> {
    mbox.send(PrivateMsg::ResetTopology).await
}

async fn handle_gossip(
    mut internal_mbox: MessageBox<PrivateMsg>,
    mut network_mbox: MessageBox<NetworkMsg>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (reply_handle, reply_future) = intercom::unary_reply();
    internal_mbox
        .send(PrivateMsg::GetGossips(reply_handle))
        .await?;
    for (peer, gossip) in reply_future.await? {
        network_mbox
            .send(NetworkMsg::Propagate(PropagateMsg::Gossip(peer, gossip)))
            .await?;
    }
    Ok(())
}

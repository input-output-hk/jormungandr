Jörmungandr network capabilities are split into:

1. the REST API, used for informational queries or control of the node;
2. the gRPC API for blockchain protocol exchange and participation;

Here we will only review the gRPC API as the REST API is described in another
chapter: [go to the REST documentation](../quickstart/03_rest_api.md)

# The protocol

The protocol is based on [`gRPC`] that combines commonly used protocols like HTTP/2 and RPC.
More precisely, Jörmungandr utilises.

This choice was made because  [`gRPC`] is already widely supported around the world because
of it's uitilization of standard protocols HTTP/2 which makes it much easier for Proxies and Firewalls to recognise
the protocol and permit the traffic.

## Type of queries

The protocol allows you to send multiple types of messages between nodes:

* sync block to remote peer's _Last Block_ (`tip`).
* propose new fragments (new transactions, certificates, ...):
  this is for the fragment propagation.
* propose new blocks: for block propagation.

There are other commands that optimise the communication and synchronisation
between nodes that will be documented here in the future.

Another type of messages is the `Gossip` message. These gossip messages allow Nodes to exchange
information (gossips) about other nodes on the network, allowing for peer
discovery.

## Peer to peer

The peer 2 peer connections are established utilising multiple components:

* A multilayered topology (e.g. [Poldercast]);
* Gossiping for node discoverability;
* Subscription mechanism for event propagation;
* Security and countermeasures: (such as Topology Policy for scoring and/or
  blacklisting nodes);

### Multilayered topology

As described in the [Poldercast] paper, our network topology is
built on multiple layers that allow for granular control of it's behavior. In 
practice this means a node will have different groups of nodes that it connects to 
based on different algorithms, each of these groups are a subset of the whole 
known list of nodes.

In short we have:

* The rings layer selects a predecessor(s) and a successor(s) for each
  topic (Fragment or Blocks);
* The Vicinity layer will select nodes that have similar interests;
* The Cyclon layer, will select nodes randomly.

However, we keep the option open to remove some of these layers or to add new
ones, such as:

* A layer to allow privilege connections between stake pools;
* A layer for the user's whitelist, a list of nodes the users considered
  trustworthy and that we could use to check in the current state of the
  network and verify the user's node is not within a long running fork;

### Gossiping

Gossiping is the process used for peer discovery. It allows two things:

1. For any nodes to advertise themselves as discoverable;
2. To discover new nodes via exchanging a list of nodes (gossips);

The gossips are selected by the different layers of the multilayered topology.
For the Poldercast modules, the gossips are selected just as in the paper.
Additional modules may select new nodes in the gossip list or may decide to
not add any new information.

### Subscription mechanism

Based on the multilayered topology, the node will open multiplexed and
bi-directional connections (thanks to industry standard [`gRPC`], this comes for
free). These bi-directional connections are used to propagate events such as:

* Gossiping events, when 2 nodes exchange gossips for peer discovery;
* Fragment events, when a node wants to propagate a new fragment to other nodes;
* Block events, when a node wants to propagate a new block creation event


### Security and countermeasures

In order to facilitate the handling of unreachable nodes or of misbehaving ones
we have built a node policy tooling. This is constructed via 2 mechanisms:
collecting connectivity statuses and blockchain status for each node. The policy
can then be tuned over the collected data to apply some parameters when
connecting to a given node, as well as banning nodes from our topology.

For each node, the following data is collected:

Connection statuses:

* The failed connection attempts and when it happened;
* Latency
* Last message used per topic item (last time a fragment has been received from
  that node, last time a block has been received from that node…)

Blockchain level info:

* Faults (e.g. trying to send an invalid block)
* Contributions in the network
* Their blockchain status (e.g. tips)

### Policy

The p2p policy provides some more fine control on how to handle nodes flagged
as not behaving as expected (see the list of data collected).

It currently works as a 3 levels: possible contact, quarantined, forgotten.
Each new gossip will create a new entry in the list of possible contact. Then
the policy, based on the logged data associated to this node, may decide to put
this node in quarantine for a certain amount of time. At the end of this time
the node may decide one of the following: keep it quarantined, make it a
possible contact again or forget about it.

The changes from one level to another is best effort only. Applying the policy
may be costly so the node applies the policy only on the node it is interested
about (a gossip update or when reporting an issue against a node). This
guarantees that the node does not spend too much time policing its database.
And it also makes sure that only the nodes of interest are up to date. However
it is possible for the node to choose, at a convenient time, to policy the whole
p2p database. This is not enforced by the protocol.

| Disposition | Description |
|:------------|:------------|
| available   | Node is available for the p2p topology for view selection and gossips. |
| quarantined | Node is not available for the p2p topology for view selection or gossips. After a certain amount of time, if the node is still being gossiped about, it will be moved to available. |
| forgotten   | A node forgotten is simply removed from the whole p2p database. However, if the node is still being gossiped about it will be added back as available and the process will start again. |

[Poldercast]: https://hal.inria.fr/hal-01555561/document
[`gRPC`]: https://www.grpc.io

Jörmungandr network capabilities are split into:

1. the REST API, used for informational queries or control of the node;
2. the gRPC API for blockchain protocol exchange and participation;

Here we will only talk of the later, the REST API is described in another
chapter already: [go to REST documentation](../quickstart/03_rest_api.md)

# The protocol

The protocol is based on commonly used in the industry tools: HTTP/2 and RPC.
More precisely, Jörmungandr utilises [`gRPC`](https://www.grpc.io).

This choice has been made for it is already widely supported across the world,
it is utilising HTTP/2 which makes it easier for Proxy and Firewall to recognise
the protocol and allow it.

## Type of queries

The protocol allows to send multiple type of messages between nodes:

* sync block to remote peer _Last Block_ (`tip`).
* propose new fragment (new transactions, certificates, ...):
  this is for the fragment propagation.
* propose new blocks: for the block propagation.

There are other commands to optimise the communication and synchronisation
between nodes.

Another type of messages is the `Gossip` message. It allows Nodes to exchange
information (gossips) about other nodes on the network, allowing the peer
discovery.

## Peer discovery

Peer discovery is done via [`Poldercast`](https://hal.inria.fr/hal-01555561/document)'s Peer to Peer (P2P) topology
construction. The idea is to allow the node to participate actively into
building the decentralized topology of the p2p network.

This is done through gossiping. This is the process of sharing with others
topology information: who is on the network, how to reach them and what are
they interested about.

In the poldercast paper there are 3 different modules implementing 3 different
strategies to select nodes to gossip to and to select the gossiping data:

1. Cyclon: this module is responsible to add a bit of randomness in the gossiping
   strategy. It also prevent nodes to be left behind, favouring contacting Nodes
   we have the least used;
2. Vicinity: this module helps with building an interest-induced links between
   the nodes of the topology. Making sure that nodes that have common interests
   are often in touch.
3. Rings: this module create an oriented list of nodes. It is an arbitrary way to
   link the nodes in the network. For each topics, the node will select a set of
   close nodes.


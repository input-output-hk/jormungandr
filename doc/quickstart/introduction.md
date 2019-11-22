# Quickstart

The rust node comes with tools and help in order to quickly start
a node and connect to the blockchain.

It is compatible with most platforms and it is pre-packaged for some
of them.

Here we will see how to install `jormungandr` and its helper `jcli`
and how to connect quickly to a given blockchain.

There are three posible ways you can start jormungandr.

## As a passive node in an existing network

As described [here](./02_passive_node.md).

The passive Node is the most common type of Node on the network. It can be used to download the blocks and broadcast transactions to peers, but it
doesn't have cryptographic materials or any mean to create blocks.
This type of nodes are mostly used for wallets, explorers or relays.

## As a node generating blocks in an existing network

The network could be running either bft or genesis consensus. In the former case the node must have the private key of a registered as a slot leader, while for the latter the private keys of a registered stake pool are needed.

More information [here](./05_leader_candidate.md)

## Creating your own network

This is similar to the previous case, but configuring a genesis file is needed. Consult the [Advanced section](../advanced/introduction.md) for more information on this procedure.

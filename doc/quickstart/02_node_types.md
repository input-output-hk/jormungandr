There are three posible ways you can start jormungandr.

# As a passive node in an existing network

As described [here](./node_types/01_passive_node.md).  

The passive Node is the most common type of Node on the network. It can be used to download the blocks and broadcast transactions to peers, but it
doesn't have cryptographic materials or any mean to create blocks.
This type of nodes are mostly used for wallets, explorers or relays.

# As a node generating blocks in an existing network

The network could be running either bft or genesis consensus. In the former case the node must have the private key of a registered as a slot leader, while for the latter the private keys of a registered stake pool are needed. 

More information [here](./node_types/02_generating_blocks.md)

# Creating your own network

This is similar to the previous case, but configuring a genesis file is needed. Consult the [Advanced section](../advanced/introduction.md) for more information on this procedure.
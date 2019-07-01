# How to start a node as a leader candidate

## Gathering data

Like in the passive node case, two things are needed to connect to an existing network

1. the hash of the **genesis block** of the blockchain, this will be the source
   of truth of the blockchain. It is 64 hexadecimal characters.
2. the **trusted peers** identifiers and access points.

The node configuration could be the same as that for [running a passive node](./01_passive_node.md). 

There are some differences depending if you are connecting to a network running a genesis or bft consensus protocol.

### Connecting to a genesis blockchain

#### Registering a stake pool

In order to be able to generate blocks in an existing genesis network, a [registered stake pool](../../stake_pool/registering_stake_pool) is needed.

#### Creating the secrets file

Put the node id and private keys in a yaml file in the following way:  

##### Example

filename: _node_secret.yaml_

```yaml
genesis:
  sig_key: Content of stake_pool_kes.prv file
  vrf_key: Content of stake_pool_vrf.prv file
  node_id: Content of stake_pool.id file
```

#### Starting the node 

```sh
jormungandr --genesis-block-hash asdf1234... --config config.yaml --secret node_secret.yaml
```

_The 'asdf1234...' part should be the actual block0 hash of the network_

### Connecting to a BFT blockchain

In order to generate blocks, the node should be registered as a slot leader in the network and started in the following way.

## The secret file

Put secret key in a yaml file, e.g. `node_secret.yaml` as follows:

```yaml
bft:
 signing_key: ed25519_sk1kppercsk06k03yk4qgea....
```

where signing_key is a private key associated to the public id of a slot leader.

### Starting the node

```sh
jormungandr --genesis-block asdf1234... --config node.config --secret node_secret.yaml
```

_The 'asdf1234...' part should be the actual block0 hash of the network_
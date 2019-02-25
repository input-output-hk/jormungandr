# Full Node

> Just because you call something a blockchain, that doesn't mean you aren't subject to normal engineering laws.

## Internal Design


Glossary:

* **blockchains**: the current blockchain and possibly different known forks.
* **clock**: general time tracking to know the time in blockchain unit (epoch/slot)
* **tip**: the current fork that is considered the correct one, related to consensus algorithm.

## Tasks.

Each node runs several tasks. Task is a process with a clearly defined interface
that abstracts a particular task.

General tasks:

* **Network task**: handle new connections, and perform lowlevel queries.
  It does queries parsing and routing them to the other tasks: block,
  client or transaction tasks.

* **Block task**: handles blocks reception from other nodes and the leadership
  thread. The blocks can be external and internal. External block (...), and
  internal block (...).
  When the task receives an external block it validates the block. If validation
  succeeds then the task appends blocks to the blockchain and checks if the tip
  needs any changes.
  When the task receives an internal block it does the same actions except for
  block validation. And then broadcasts the change of the tip to the network
  thread.

* **Leadership task**: waits for each new slot, evaluates if this node is
  a slot leader. In case if it is, the task creates a new block
  (with a set of known transactions) referencing the latest known
  and agreed block in the blockchain. Then the task sends it to the block
  thread for processing.

* **Client task**: receives block header/body queries. This task is in charge
  of in accord [!!!] with the blockchains, reply to the client.

* **Transaction task**: receives new transactions from the network,
  validates transaction and handle duplicates.
  Also the broadcast to other nodes new (valid) transaction received.

![Internal Architecture](/.architecture-1.png?raw=true "Internal Architecture")


## How To Use

In order to use jormungandr you need to configure your blockchain and
configure your node.
In order to configure a blockchain you should have a genesis file. If
you want to create a new blockchain you can create a new genesis file.
See 'create your genesis file' section.

Then you need to configure your nodes, see 'node configuration section'.

After configuring the blockchain and the node you can start one,
see 'starting the node' section.

### Create your genesis file

In order to do so you should create:

* the genesis data : That is the data that will be used to initialise the
  protocol properties (like the initial UTxOs);
* the protocol properties;

Run following command to generate your `genesis.yaml` file:

```
jormungandr init \
    --initial-utxos=ca1qvqsyqcyq5rqwzqfpg9scrgwpugpzysnzs23v9ccrydpk8qarc0jqxuzx4s@999999999
```

Running the command above will generate (WARNING: this is temporary, the genesis data format will be updated):

```yaml
---
start_time:
  secs_since_epoch: 1550822014
  nanos_since_epoch: 930587000
slot_duration:
  secs: 15
  nanos: 0
epoch_stability_depth: 2600
initial_utxos:
  - address: ca1qvqsyqcyq5rqwzqfpg9scrgwpugpzysnzs23v9ccrydpk8qarc0jqxuzx4s
    value: 999999999
```

You store this in a genesis.yaml file, you can the modify/tune your genesis data.

Configuration fields meaning:
  - *start_time*: when the blockchain starts
  - *slot_duration*: amount of time each slot is running.
  - *epoch_stability_depth*: allowed size of the fork (in blocks).
  - *initial_utuxos*:

### Node Configuration

Example of node config:

```
bft:
  constants:
    t: 10
  leaders:
    - 482ec7835412bcc18ca5c1f15baef53e0d62092fe1bbf40ea30fac895fd0f98c3b009cfd62715a5b871aabf5d603bec5aa5c8b3eae537fb254dd83ef88950d7d
    - b77f6ed6edbb0a63e09764ccaf2bb6bb5cdc8e54ce1bab6aeccacb98848dfe01b77a9be9254a0f2d103953264df9b7957d8e61608b196723c109c28c89c1bb1e
grpc_listen:
       - "127.0.0.1:8081"
storage: "/tmp/storage"
logger:
  verbosity: 1
  format: json
```

Fields description:

  - *bft.constants.t*: (to be removed)
  - *bft.leaders*: public keys of the nodes.
  - *grpc_listen*: (optional) addresses of the other
      nodes that are connected using grpc protocol.
  - *storage*: (optional) path to the storage
  - *logger*: (optional) logger configuration,
     - *verbosity*: 0 - warning, 1 - info, 2 -debug, 3 and above - trace
     - *format*: log output format - plain or json.

### Starting the node

If you are not a leader node, then you can start the jormundandr with:

```
jormungandr start --genesis-config genesis.yaml \
  --config example.config \
  --without-leadersip
```

In order to start a leader node you need to generate key pairs using
`cardano-cli`:

```
cardano-cli debug generate-xprv key.xprv
cardano-cli debug xprv-to-xpub key.xprv key.xpub
```

Then you should start node using:

```
jormungandr start --genesis-config genesis.yaml \
  --config example.config \
  --secret key.xprv
```

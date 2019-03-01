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
    --initial-utxos=ca1qvqsyqcyq5rqwzqfpg9scrgwpugpzysnzs23v9ccrydpk8qarc0jqxuzx4s@999999999 \
    --obft-leader=5b66c12d1aa6986d9c37b7bf905826a95db4b1c28e7c24fbaeb6ec277f75bd59 \
    --obft-leader f976bd9025d8c26928479ebdd39c12ac2cf5ce73f6534648a78ddc0da2f57794
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
obft_leaders:
  - 5b66c12d1aa6986d9c37b7bf905826a95db4b1c28e7c24fbaeb6ec277f75bd59
  - f976bd9025d8c26928479ebdd39c12ac2cf5ce73f6534648a78ddc0da2f57794

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
grpc_listen:
       - "127.0.0.1:8081"
storage: "/tmp/storage"
logger:
  verbosity: 1
  format: json
rest:
  listen: "127.0.0.1:8443"
  pkcs12: "example.p12"
  prefix: "api"
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
  - *rest*: (optional) configuration of the rest endpoint.
     - *listen*: listen address
     - *pkcs12*: certificate file
     - *prefix*: (optional) api prefix

### Starting the node

If you are not a leader node, then you can start the jormundandr with:

```
jormungandr start --genesis-config genesis.yaml \
  --config example.config \
  --without-leadersip
```

In order to start a leader node you need to generate key pairs using
`jormungandr`:

```
jormungandr generate-keys
signing_key: 90167eccc5db6ab75c643e33901ec727be847aa51f16890df06ec6fa401e9958
public_key: 77d0edad4553bbb66115ce1ed78ca0e752534a0d2faa707d4356ea567a586475
```

`singing_key` is your private key you can put it in key.xprv file,
note that there should be no EOL in that file. If you expect your
node to be a leader, put your public_key in the `genesis.yaml` leader.

Then you should start node using:

```
jormungandr start --genesis-config genesis.yaml \
  --config example.config \
  --secret key.xprv
```

# License

This project is licensed under either of the following licenses:

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

Please choose the licence you want to use.

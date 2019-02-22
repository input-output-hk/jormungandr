# Full Node

> Just because you call something a blockchain, that doesn't mean you aren't subject to normal engineering laws.

## Internal Design


glossary:

* blockchains: the current blockchain and possibly different known forks.
* clock: general time tracking to know the time in blockchain unit (epoch/slot)
* tip: the current fork that is considered the correct one, related to consensus algorithm.

General tasks:

* Network task: Handle new connections, and lowlevel queries. mostly parsing and routing them to
  block, client or transaction tasks.

* Block task: Handle all the blocks reception from nodes and leadership thread.
  On reception of external blocks, validate the block, and on succesful validation
  append to the blockchains, check if the blockchain tip need change. On internal block
  reception, same except without need for validation, broadcast change of tip back to network thread

* Leadership task: Wait for each new slot, and evaluate whether or not this node is
  a slot leader. If yes, then create a new block (with a set of known
  transactions) referencing the latest known and agreed block in the blockchain,
  then send it to the block thread for processing (appending to blockchain structure, then broadcasting)

* Client task: receive block header/body queries (e.g. Get Block 1 to 2000), and is in charge
  of in accord with the blockchains, reply to the client.

* Transaction task: receive new transaction from the network, validate transaction and handle duplicates.
  Also broadcast to other nodes new (valid) transaction received.

![Internal Architecture](/.architecture-1.png?raw=true "Internal Architecture")


## How To Use

### Create your genesis file

if you don't have a genesis file yet but you want to create a new bkockchain
you will need to create:

* the genesis data : That is the data that will be used to initialise the
  protocol properties (like the initial UTxOs);
* the protocol properties;

There is simple command you can run to generate your genesis.yaml file:

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

## Node Configuration

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

`legacy_listen`, `grpc_listen`, `storage` and `logger` fields are optional and can be omitted.
Verbosity levels are descripbed as an integer where 0 - warning, 1 - info, 3 - debug, 4 and above - trace.
Format is one of "json" or "plain".

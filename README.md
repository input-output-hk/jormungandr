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


## Config

Example of node config:

```
bft:
  constants:
    t: 10
  leaders:
    - 482ec7835412bcc18ca5c1f15baef53e0d62092fe1bbf40ea30fac895fd0f98c3b009cfd62715a5b871aabf5d603bec5aa5c8b3eae537fb254dd83ef88950d7d
    - b77f6ed6edbb0a63e09764ccaf2bb6bb5cdc8e54ce1bab6aeccacb98848dfe01b77a9be9254a0f2d103953264df9b7957d8e61608b196723c109c28c89c1bb1e
legacy_listen:
       - "127.0.0.1:8080"
grpc_listen:
       - "127.0.0.1:8081"
legacy_peers:
       - "127.0.0.1:9000"
storage:
       - "/tmp/storage"
logger:
  verbosity: 1
  format: json
```

`legacy_listen`, `grpc_listen`, `storage` and `logger` fields are optional and can be omitted.
Verbosity levels are descripbed as an integer where 0 - warning, 1 - info, 3 - debug, 4 and above - trace.
Format is one of "json" or "plain".

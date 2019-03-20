# Internal Design

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

![Internal Architecture](.architecture-1.png?raw=true "Internal Architecture")

# Blockchain concepts

## Time

Slots represent the basic unit of time in the blockchain, and at each slot
a block could be present.

Consecutive slots are grouped into epochs, which have updatable size defined
by the protocol.

## Fragments

Fragments are part of the blockchain data that represent all the possible
events related to the blockchain health (e.g. update to the protocol), but
also and mainly the general recording of information like transactions and
certificates.

## Blocks

Blocks represent the spine of the blockchain, safely and securely linking
blocks in a chain, whilst grouping valid fragments together.

Blocks are composed of 2 parts:

* The header
* The content

The header link the content with the blocks securely together, while the
content is effectively a sequence of fragments.

## Blockchain

The blockchain is the general set of rules and the blocks that are periodically created.
Some of the rules and settings, can be changed dynamically in the system by updates,
while some other are hardcoded in the genesis block (first block of the blockchain).

```
    +-------+      +-------+
    |Genesis+<-----+Block 1+<--- ....
    |Header |      |Header |
    +---+---+      +---+---+
        |              |
    +---v---+      +---v---+
    |Genesis|      |Block 1|
    |Content|      |Content|
    +-------+      +-------+
```

## Consensus

The node currently support the following consensus protocol:

* Ouroboros BFT (OBFT)
* Ouroboros Genesis-Praos

Ouroboros BFT is a simple Byzantine Fault Tolerant (BFT) protocol where the
block makers is a known list of leaders that successively create a block and
broadcast it on the network.

Ouroboros Genesis Praos is a proof of stake (PoS) protocol where the block
maker is made of a lottery where each stake pool has a chance proportional to
their stake to be elected to create a block. Each lottery draw is private to
each stake pool, so that the overall network doesn't know in advance who can
or cannot create blocks.

In Genesis-Praos slot time duration is constant, however the frequency of 
creating blocks is not stable, since the creation of blocks is a probability 
that is linked to the stake and consensus_genesis_praos_active_slot_coeff.

**Note**: In Genesis-Praos, if there is no stake in the system, no blocks will be 
created anymore starting with the next epoch.

## Leadership

The leadership represent in abstract term, who are the overall leaders of the
system and allow each individual node to check that specific blocks are
lawfully created in the system.

The leadership is re-evaluated at each new epoch and is constant for the
duration of an epoch.

## Leader

Leader are an abstraction related to the specific actor that have the ability
to create block; In OBFT mode, the leader just the owner of a cryptographic
key, whereas in Genesis-Praos mode, the leader is a stake pool.

## Transaction

Transaction forms the cornerstone of the blockchain, and is one type of fragment and also the most frequent one.

Transaction is composed of inputs and outputs; On one side, the inputs represent coins being spent, and on the other side the outputs represent coins being received.

```
    Inputs         Alice (80$)        Bob (20$)
                        \             /
                         \           /
                          -----------
                                100$
                             --------- 
                            /         \
    Outputs            Charlie (50$)  Dan (50$)
```

Transaction have fees that are defined by the blockchain settings and the following invariant hold:

\\[ \sum Inputs = \sum Outputs + fees \\]

Transaction need to be authorized by each of the inputs in the transaction by their respective witness.
In the most basic case, a witness is a cryptographic signature, but depending on the type of input can the type of witness vary.

## Accounting

The blockchain has two methods of accounting which are interoperable:

* Unspent Transaction Output (UTXO)
* Accounts

UTXO behaves like cash/notes, and work like fixed denomination ticket that are cumulated. This is the accounting model found in Bitcoin. A UTXO is uniquely reference by its transaction ID and its index.

Accounts behaves like a bank account, and are simpler to use since exact amount can be used. This is the accounting model found in Ethereum. An account is uniquely identified by its public key.

Each inputs could refer arbitrarily to an account or a UTXO, and similarly each outputs could refer to an account or represent a new UTXO.

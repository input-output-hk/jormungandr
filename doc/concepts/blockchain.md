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
broadcast it on the network

Ouroboros Genesis Praos is a proof of stake (PoS) protocol where the block
makers is a lottery where each stake pool has a chance proportional to their
stake to be elected to create a block. Each lottery draw is private to each
stake pool, so that the overall network doesn't know in advance who can
or cannot create blocks.

## Leadership

The leadership represent in abstract term, who are the overall leaders of the
system and allow each individual node to check that specific blocks are
lawfully created in the system.

The leadership is re-evaluated at each new epoch and is constant for the
duration of an epoch.

## Leader

Leader are an abstration related to the specific actor that have the ability
to create block; In OBFT mode, the leader just the owner of a cryptographic
key, whereas in GenesisPraos mode, the leader is a stake pool.

## UTxO
<explain concept here>

## Accounts
<explain concept here>

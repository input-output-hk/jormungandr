# MIGRATION

This is the migration plan for current cardano blockchain (henceforth refered as
legacy) to jormungandr style state and formats.

## Vocabulary

* Block Zero: first/genesis block of the blockchain.

## Description

It's paramount for all users from the legacy chain to find their precious data
after the migration. Also as one secondary consideration, the users need not to
be aware, as much as possible of the transition, apart from requiring new or
updated software capable of handling the new formats and processes. Lastly,
it would be useful to provide some kind of cryptographic continuinity of the
chains, increasing assurances during transition.

The first thing that need consideration is the legacy utxos. We need the ability
to take the latest known state of coin distribution and transfer this as is
to the new state order.

The settings of the legacy chain, are automatically superseded by the new
settings mandatory in block zero, so there's no need to keep any related data.

The heavy/light delegation certificates are also superseded by either the BFT
leaders or the Genesis-Praos stake pools defined explicitely in block zero.

From a user experience and offering continuinity of history, it would be
preferable to start the chain initial date at the end of the legacy one. This
way the user can still refer to historical transaction in the legacy era of the
chain without seeing similar block date on two different era.

Finally it's important to provide as much guarantee as possible of the
transition, and hence knowing the hash of last block of the legacy chain on
"the other side", would allow some validation mechanism. Block 0 content being
a trusted data assumption, having the previous hash embedded directly inside,
reconstruct the inherent chain of trust of a blockchain cheaply.

## Mechanisms

To support this, the following continuinity mechanisms are thus available:

* blockchain continuity: the ability to embed inside block zero of the
  chain an arbitrary hash of data, representing the last block of the legacy chain.
* user experience: block zero choice of start of epoch (e.g. starting the new chain at epoch 129).
* legacy funds: A sequence of legacy address and their associated values

Note: On the blockchain continuity, we decided to store the hash as an opaque
blob of data in the content, instead of using the normal blockchain
construction of the previous hash. Using the previous hash, would have made
the start condition of the blockchain harder to detect compared to the
sentinel 0 value currently in place and would have forced to have an
identical hash size by construction.

The legacy funds are automatically assigned a new transaction-id / index in the
new system, compared to whichever computed transaction-id / index in the legacy
chain. This new transaction-id is computed similarly from normal transaction
in the new system, and no special case has been added to support this. However
the legacy address is stable across this transition, allowing user to
find their funds on whichever address it was left, at the value it was left.

## Transaction

To clearly break from the past, the old funds are only allowed to be consumed,
leading to the old state monotonically decreasing. This also prevent
from having the old legacy address construction available in witness
or outputs.

The transaction-id/index system is the same as normal funds, so the inputs
doesn't requires any modification, however we need to distinguish the witness
since the witness on the old chain is different. This provide a clear
mechanism to distinguish the type of input (fund or old-fund).

The witness construction is similar to what is found on the old chain, an
extended public key followed by a signature.

## Mainnet-Testnet tests

Considering the risk involve in such a migration, we can repeatly
tests mock migration at arbitrary points (preferably at end of epoch).

The migration itself will be fully automated and repeadtly tested for
solidity and accuracy, and can be done with mostly off the shelf code
that we already have.

The migration will capture the latest known state and create the equivalent
genesis.yaml file mingled with the settings for the new blockchain, and
subsequently compiled into a working block0. The task itself should be
completeable in sub-second, leading to a very small window of transition.
Although to note, the block0 size is proportional to the number of state
point that is being kept; Approximately for ~200000 utxos, 13mb of
block zero will be created.

rust-cardano's chain-state is already capable to capture the latest known state,
but there's no currently any genesis generational tool for this task, although
the task remain fairly simple.

## Advantages

The net benefits is the total removal of *all* legacy constructs; The new
users or software have no need to handle any of the legacy data.

This also provide an implicit net chain "compression":

> what happened in Byron, stays in Byron.

The legacy addresses are particularly problematic for many reasons not
described here, but not providing full usage is particularly advantageous,
nonetheless since it provide a way to not have their numbers go up ever after
transition.

## Historical data

From historical purpose and bip44 wallets, we need to provide the legacy blocks.

The legacy blocks can be made available from a 3rd party service for a one-of
2.0 Gb download (approximate: all the mainnet data), for example using a
service like cardano-http-bridge which have caching and CDN capability,
leading to a very small cost for the provider of such a service.

It's also possible to provide the historical data as part of the node,
supplementing the current interface with an interface to download old data
ala cardano-http-bridge. The first option is strongly favored to cleanly
break the legacy data from the new data.

Legacy randomized wallets (e.g. Ddz addresses) will not need to download the full
history, since the legacy address contains the metadata sufficient for
recovery, so only block zero is necessary to know their own fund.

On the other hand, legacy BIP44 wallets will *need* to download the full history to be
able to recover their BIP44 state at the transition.

For wallet history of legacy wallets, the historical data will have to be
downloaded too.

For new wallet, after the transition, this historical data will not be needed whatsoever,
saving 2.0gb of download for new users.

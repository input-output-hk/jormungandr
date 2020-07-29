/*!
# Blockchain management module

this module provides a high level API to manage the blockchain and the different
state it may be. Allowing tracking different branches and to be able to make simple
informed decisions as to what is the tip of the blockchain or what is a valid block.

## Reference

the `Reference` holds the state information at the given block. It can be used to
know the blockdate, the chainlength etc... but it also links to the epoch information
such as the current leadership schedule: this is core to validate a block.

From a `Reference` it is possible to `chain` a `Block`. This means creating a new
reference with a derive state from the previous `Reference`. This operation will
do all the necessary verifications.

## EpochInfo

The `EpochInfo` is the data tied to a given Epoch: the TimeFrame, the stake
distribution, the leadership schedule, the ledger parameters (fees, etc...).
When chaining a `Block` with a `Reference`, if an epoch transition occurs,
a new `EpochInfo` will be constructed for the new epoch.

## Selection between 2 branches

In some consensus, it may be that there are multiple competing blocks for the
same slot in the blockchain. It is possible to use `select` to compare the
2 different `Reference`.

## Checkpoints

From any reference, it is possible to build `Checkpoints`. These is a list
of header ID (block's header hash) that are selected arbitrarily from the
given `Reference`. They can be used to send to the network or storage as
hints of where to start from when requiring a stream of blocks from one of
the header ID in the Checkpoints.

# The Blockchain manager

the `Blockchain` is a simple collection object that will keep track of the
different branches and blocks as well as the Tip of the blockchain. It will
garbage collect the `Reference` that are not used to prevent keeping too
much non needed data in memory.

When adding a new block, the blockchain will lookup for the best option
regarding branch management. It will first look for an existing branch
to continue it or will create a new branch if required.

If the block is added successfully, the `Event::Added` will be returned.
It contains the details as to what and how it was added. A new branch
may have been created, it may even be the new tip. It will also tell if an
epoch transition occurs: that way the application may notify the different
module with the appropriate data.
*/

pub(crate) mod block0;
mod blockchain;
mod checkpoints;
mod epoch_info;
mod reference;

pub use self::{
    blockchain::{Blockchain, Configuration, Event},
    checkpoints::Checkpoints,
    epoch_info::{EpochInfo, EpochInfoError},
    reference::{Error, Reference, Selection},
};

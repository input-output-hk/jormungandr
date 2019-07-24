/*

```text
          +------------+                     +------------+                    +------------+
          | Leadership |                     | Leadership |                    | Leadership |
          +-----+------+                     +-----+------+                    +-------+----+
                ^                                  ^                                   ^
                |                                  |                                   |
      +---------v-----^--------------+             +<------------+                +--->+--------+
      |               |              |             |             |                |             |
      |               |              |             |             |                |             |
   +--+--+         +--+--+        +--+--+       +--+--+       +--+--+          +--+--+       +--+--+
   | Ref +<--------+ Ref +<-------+ Ref +<--+---+ Ref +<------+ Ref +<---------+ Ref +<------+ Ref |
   +--+--+         +--+--+        +--+--+   ^   +--+--+       +--+--+          +---+-+       +---+-+
      |               |              |      |      |             |                 |             |
      v               v              v      |      v             v                 v             v
+-----+--+      +-----+--+       +---+----+ |   +--+-----+   +---+----+      +-----+--+       +--+-----+
| Ledger |      | Ledger |       | Ledger | |   | Ledger |   | Ledger |      | Ledger |       | Ledger |
+--------+      +--------+       +--------+ |   +--------+   +--------+      +--------+       +--------+
                                            |
                                            |
                                            |parent
                                            |hash
                                            |
                                            |         +----------+
                                            +---------+New header|
                                                      +----------+
```

When proposing a new header to the blockchain we are creating a new
potential fork on the blockchain. In the ideal case it will simply be
a new block on top of the _main_ current branch. We are adding blocks
after the other. In some cases it will also be a new branch, a fork.
We need to maintain some of them in order to be able to make an
informed choice when selecting the branch of consensus.

We are constructing a blockchain as we would on with git blocks:

* each block is represented by a [`Ref`];
* the [`Ref`] contains a reference to the associated `Ledger` state
  and associated `Leadership`.

A [`Branch`] contains a [`Ref`]. It allows us to follow and monitor
forks between different tasks of the blockchain module.

[`Ref`]: ./struct.Ref.html
[`Branch`]: ./struct.Branch.html
*/

mod branch;
mod reference;
mod reference_cache;

pub use self::{
    branch::{Branch, Branches},
    reference::Ref,
    reference_cache::RefCache,
};
use crate::{
    blockcfg::{Block, Block0Error, Leadership, Ledger, Multiverse},
    leadership::Leaderships,
    start_up::NodeStorage,
};
use chain_core::property::{Block as _, HasFragments as _};
use chain_impl_mockchain::ledger;
use chain_storage::error::Error as StorageError;
use std::time::Duration;

error_chain! {
    foreign_links {
        Storage(StorageError);
        Ledger(ledger::Error);
        Block0(Block0Error);
    }

    errors {
        PoisonedLock {
            description("lock is poisoned"),
        }

        HeaderHashAlreadyPresent {
            description("The HeaderHash is already present in the storage"),
        }
    }
}

pub struct Blockchain {
    branches: Branches,

    ref_cache: RefCache,

    multiverse: Multiverse<Ledger>,

    leaderships: Leaderships,

    storage: NodeStorage,
}

impl Blockchain {
    pub fn new(mut storage: NodeStorage, block_0: Block, ref_cache_ttl: Duration) -> Result<Self> {
        let mut multiverse = Multiverse::new();
        let mut leaderships = Leaderships::new();

        let state = Ledger::new(block_0.id(), block_0.fragments())?;
        storage.put_block(&block_0)?;
        let initial_leadership = Leadership::new(block_0.date().epoch, &state);
        let gcroot_ledger = multiverse.add(block_0.id(), state.clone());
        let gcroot_leadership = leaderships.add(
            block_0.date().epoch,
            block_0.chain_length(),
            block_0.id(),
            initial_leadership,
        );

        let branch = Branch::new(Ref::new(
            gcroot_ledger,
            gcroot_leadership,
            block_0.header.clone(),
        ));
        let mut branches = Branches::new();
        branches.add(branch);

        Ok(Blockchain {
            branches: branches,
            ref_cache: RefCache::new(ref_cache_ttl),
            multiverse: multiverse,
            leaderships: leaderships,
            storage: storage,
        })
    }
}

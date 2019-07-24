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
mod multiverse;
mod reference;
mod reference_cache;
mod storage;

pub use self::{
    branch::{Branch, Branches},
    multiverse::Multiverse,
    reference::Ref,
    reference_cache::RefCache,
    storage::Storage,
};
use crate::{
    blockcfg::{Block, Block0Error, HeaderHash, Leadership, Ledger},
    start_up::NodeStorage,
};
use chain_core::property::{Block as _, HasFragments as _};
use chain_impl_mockchain::ledger;
use chain_storage::error::Error as StorageError;
use std::time::Duration;
use tokio::prelude::*;

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

#[derive(Clone)]
pub struct Blockchain {
    branches: Branches,

    ref_cache: RefCache,

    multiverse: Multiverse<Ledger>,

    leaderships: Multiverse<Leadership>,

    storage: Storage,
}

impl Blockchain {
    pub fn new(storage: NodeStorage, ref_cache_ttl: Duration) -> Self {
        Blockchain {
            branches: Branches::new(),
            ref_cache: RefCache::new(ref_cache_ttl),
            multiverse: Multiverse::new(),
            leaderships: Multiverse::new(),
            storage: Storage::new(storage),
        }
    }

    pub fn apply_block0(&mut self, block0: Block) -> impl Future<Item = (), Error = Error> {
        let block0_header = block0.header.clone();
        let block0_id = block0_header.hash();
        let block0_date = block0_header.block_date();
        let block0_chain_length = block0_header.chain_length();

        // 1. check the block0 is not already in the storage

        let block0_ledger = Ledger::new(block0_id.clone(), block0.contents.iter())
            // TODO: handle that case
            .unwrap();
        let block0_leadership = Leadership::new(block0_date.epoch, &block0_ledger);

        // TODO: chain futures
        let block0_ledger_gcroot = self
            .multiverse
            .insert(block0_chain_length, block0_id.clone(), block0_ledger)
            .wait()
            .unwrap();
        // TODO: chain futures
        let block0_leadership_gcroot = self
            .leaderships
            .insert(block0_chain_length, block0_id.clone(), block0_leadership)
            .wait()
            .unwrap();

        let reference = Ref::new(
            block0_ledger_gcroot,
            block0_leadership_gcroot,
            block0_header,
        );

        // TODO: chain futures
        self.ref_cache
            .insert(block0_id.clone(), reference.clone())
            .wait()
            .unwrap();

        let branch = Branch::new(reference);
        // TODO: chain futures
        self.branches.add(branch).wait().unwrap();

        // TODO: store block0 in storage

        future::ok(())
    }
}

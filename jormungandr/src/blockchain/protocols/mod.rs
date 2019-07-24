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

    pub fn push(&mut self, block: Block) -> Result<Self> {
        unimplemented!()
    }

    pub fn get_branch_including(&mut self, header_hash: HeaderHash) -> Option<Branch> {
        unimplemented!()
    }
}

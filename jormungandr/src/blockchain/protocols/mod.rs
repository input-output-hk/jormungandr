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
    blockcfg::{Block, Block0Error, Header, HeaderHash, Leadership, Ledger},
    start_up::NodeStorage,
};
use chain_impl_mockchain::ledger;
use chain_storage::error::Error as StorageError;
use std::{convert::Infallible, time::Duration};
use tokio::prelude::*;

error_chain! {
    foreign_links {
        Storage(StorageError);
        Ledger(ledger::Error);
        Block0(Block0Error);
    }

    errors {
        Block0InitialLedgerError {
            description("Error while creating the initial ledger out of the block0")
        }

        Block0AlreadyExists {
            description("Block0 already exists in the storage")
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

    /// create and store a reference of this leader to the new
    fn create_and_store_reference(
        &mut self,
        header_hash: HeaderHash,
        header: Header,
        ledger: Ledger,
        leadership: Leadership,
    ) -> impl Future<Item = Ref, Error = Infallible> {
        let chain_length = header.chain_length();

        let leaderships = self.leaderships.clone();
        let multiverse = self.multiverse.clone();
        let ref_cache = self.ref_cache.clone();

        multiverse
            .insert(chain_length, header_hash, ledger)
            .and_then(move |ledger_gcroot| {
                leaderships
                    .insert(chain_length, header_hash, leadership)
                    .map(|leadership_gcroot| (ledger_gcroot, leadership_gcroot))
            })
            .and_then(move |(ledger_gcroot, leadership_gcroot)| {
                let reference = Ref::new(ledger_gcroot, leadership_gcroot, header);
                ref_cache
                    .insert(header_hash, reference.clone())
                    .map(|()| reference)
            })
    }

    pub fn apply_block0(&mut self, block0: Block) -> impl Future<Item = (), Error = Error> {
        let block0_clone = block0.clone();
        let block0_header = block0.header.clone();
        let block0_id = block0_header.hash();
        let block0_id_1 = block0_header.hash();
        let block0_date = block0_header.block_date().clone();

        let mut self1 = self.clone();
        let mut branches = self.branches.clone();
        let mut storage_store = self.storage.clone();

        self.storage
            .block_exists(block0_id.clone())
            .map_err(|e| Error::with_chain(e, "Cannot check if block0 is in storage"))
            .and_then(|existence| {
                if !existence {
                    future::err(ErrorKind::Block0AlreadyExists.into())
                } else {
                    future::ok(())
                }
            })
            .and_then(move |()| {
                // we lift the creation of the ledger in the future type
                // this allow chaining of the operation and lifting the error handling
                // in the same place
                Ledger::new(block0_id_1, block0.contents.iter())
                    .map(future::ok)
                    .map_err(|err| Error::with_chain(err, ErrorKind::Block0InitialLedgerError))
                    .unwrap_or_else(future::err)
            })
            .map(move |block0_ledger| {
                let block0_leadership = Leadership::new(block0_date.epoch, &block0_ledger);
                (block0_ledger, block0_leadership)
            })
            .and_then(move |(block0_ledger, block0_leadership)| {
                self1
                    .create_and_store_reference(
                        block0_id,
                        block0_header,
                        block0_ledger,
                        block0_leadership,
                    )
                    .map_err(|_: Infallible| unreachable!())
            })
            .map(Branch::new)
            .and_then(move |branch| branches.add(branch).map_err(|_: Infallible| unreachable!()))
            .and_then(move |()| {
                storage_store
                    .put_block(block0_clone)
                    .map_err(|e| Error::with_chain(e, "Cannot put block0 in storage"))
            })
    }
}

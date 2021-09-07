pub mod chain_storable;
mod endian;
pub mod error;
mod helpers;
pub mod pagination;
mod pair;
pub mod schema;
mod state_ref;
// mod tally;

use self::endian::B64;
use self::error::ExplorerError;
use chain_core::property::Block as _;
use chain_impl_mockchain::block::Block;
use chain_impl_mockchain::block::HeaderId as HeaderHash;
use sanakirja::{btree, direct_repr, Storable, UnsizedStorable};
use std::path::Path;
use std::sync::Arc;

pub(crate) type P<K, V> = btree::page::Page<K, V>;
type Db<K, V> = btree::Db<K, V>;

type SanakirjaMutTx = ::sanakirja::MutTxn<Arc<::sanakirja::Env>, ()>;
type SanakirjaTx = ::sanakirja::Txn<Arc<::sanakirja::Env>>;

// A Sanakirja pristine.
#[derive(Clone)]
pub struct Pristine {
    pub env: Arc<::sanakirja::Env>,
}

impl Pristine {
    pub fn new<P: AsRef<Path>>(name: P) -> Result<Self, ExplorerError> {
        Self::new_with_size(name, 1 << 20)
    }

    pub fn new_with_size<P: AsRef<Path>>(name: P, size: u64) -> Result<Self, ExplorerError> {
        let env = ::sanakirja::Env::new(name, size, 2);
        match env {
            Ok(env) => Ok(Pristine { env: Arc::new(env) }),
            Err(e) => Err(ExplorerError::SanakirjaError(e)),
        }
    }

    pub fn new_anon() -> Result<Self, ExplorerError> {
        Self::new_anon_with_size(1 << 20)
    }

    pub fn new_anon_with_size(size: u64) -> Result<Self, ExplorerError> {
        Ok(Pristine {
            env: Arc::new(::sanakirja::Env::new_anon(size, 2)?),
        })
    }
}

#[derive(Clone)]
pub struct Explorer {
    pub db: ExplorerDb,
}

#[derive(Clone)]
pub struct ExplorerDb {
    pristine: Pristine,
}

#[derive(Clone)]
pub struct Settings {
    /// This is the prefix that's used for the Address bech32 string representation in the
    /// responses (in the queries any prefix can be used). base32 serialization could
    /// also be used, but the `Address` struct doesn't have a deserialization method right
    /// now
    pub address_bech32_prefix: String,
}

pub enum OpenDb {
    Initialized {
        db: ExplorerDb,
        last_stable_block: u32,
    },
    NeedsBootstrap(NeedsBootstrap),
}

pub struct NeedsBootstrap(Pristine);

impl NeedsBootstrap {
    pub fn add_block0(self, block0: Block) -> Result<ExplorerDb, ExplorerError> {
        let pristine = self.0;

        let mut mut_tx = pristine.mut_txn_begin()?;

        let parent_id = block0.parent_id();
        let block_id = block0.id();

        mut_tx.add_block0(&parent_id.into(), &block_id.into(), block0.contents.iter())?;

        mut_tx.commit()?;

        Ok(ExplorerDb { pristine })
    }
}

pub struct Batch {
    txn: schema::MutTxn<()>,
}

impl Batch {
    /// Try to add a new block to the indexes, this can fail if the parent of the block is not
    /// processed. This doesn't perform any validation on the given block and the previous state,
    /// it is assumed that the Block is valid
    /// IMPORTANT: this call is blocking, any calls to it should be encapsulated in a threadpool
    pub fn apply_block(&mut self, block: Block) -> Result<(), ExplorerError> {
        self.txn.add_block(
            &block.parent_id().into(),
            &block.id().into(),
            block.chain_length().into(),
            block.header.block_date().into(),
            block.fragments(),
        )?;

        Ok(())
    }

    /// IMPORTANT: this call is blocking, any calls to it should be encapsulated in a threadpool
    pub fn commit(self) -> Result<(), ExplorerError> {
        self.txn.commit()
    }
}

impl ExplorerDb {
    pub fn open() -> Result<OpenDb, ExplorerError> {
        let pristine = Pristine::new("explorer-storage")?;

        let txn = pristine.txn_begin();

        match txn {
            Ok(txn) => Ok(OpenDb::Initialized {
                last_stable_block: txn.get_last_stable_block().get(),
                db: ExplorerDb { pristine },
            }),
            Err(ExplorerError::UnitializedDatabase) => {
                Ok(OpenDb::NeedsBootstrap(NeedsBootstrap(pristine)))
            }
            Err(e) => Err(e),
        }
    }

    /// Try to add a new block to the indexes, this can fail if the parent of the block is not
    /// processed. This doesn't perform any validation on the given block and the previous state,
    /// it is assumed that the Block is valid
    pub async fn apply_block(&self, block: Block) -> Result<(), ExplorerError> {
        let pristine = self.pristine.clone();
        tokio::task::spawn_blocking(move || {
            let mut_tx = pristine.mut_txn_begin()?;

            let mut batch = Batch { txn: mut_tx };

            batch.apply_block(block)?;

            batch.commit()?;

            Ok(())
        })
        .await
        .unwrap()
    }

    pub async fn start_batch(&self) -> Result<Batch, ExplorerError> {
        let pristine = self.pristine.clone();

        tokio::task::spawn_blocking(move || {
            let mut_tx = pristine.mut_txn_begin()?;

            Ok(Batch { txn: mut_tx })
        })
        .await
        .unwrap()
    }

    pub async fn get_txn(&self) -> Result<schema::Txn, ExplorerError> {
        let pristine = self.pristine.clone();
        tokio::task::spawn_blocking(move || {
            let txn = pristine.txn_begin()?;

            Ok(txn)
        })
        .await
        .unwrap()
    }

    pub async fn set_tip(&self, hash: HeaderHash) -> Result<bool, ExplorerError> {
        let pristine = self.pristine.clone();
        tokio::task::spawn_blocking(move || {
            let mut mut_tx = pristine.mut_txn_begin()?;

            let status = mut_tx.set_tip(&hash.into())?;

            if status {
                mut_tx.commit()?;
            }

            Ok(status)
        })
        .await
        .unwrap()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct SeqNum(B64);

direct_repr!(SeqNum);

impl SeqNum {
    pub const MAX: SeqNum = SeqNum(B64(zerocopy::U64::<byteorder::BigEndian>::MAX_VALUE));
    pub const MIN: SeqNum = SeqNum(B64(zerocopy::U64::<byteorder::BigEndian>::ZERO));

    pub fn new(n: u64) -> Self {
        Self(B64::new(n))
    }

    pub fn next(self) -> SeqNum {
        Self::new(self.0.get() + 1)
    }
}

impl From<SeqNum> for u64 {
    fn from(n: SeqNum) -> Self {
        n.0.get()
    }
}

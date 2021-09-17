pub mod chain_storable;
mod endian;
pub mod error;
mod helpers;
mod pair;
pub mod schema;
mod state_ref;

use self::endian::B64;
use self::error::DbError;
use chain_core::property::Block as _;
use chain_impl_mockchain::block::Block;
use chain_impl_mockchain::block::HeaderId as HeaderHash;
use sanakirja::{btree, direct_repr, Storable, UnsizedStorable};
use std::path::Path;
use std::sync::Arc;
use zerocopy::{AsBytes, FromBytes};

pub(crate) type P<K, V> = btree::page::Page<K, V>;
type Db<K, V> = btree::Db<K, V>;

type SanakirjaMutTx = ::sanakirja::MutTxn<Arc<::sanakirja::Env>, ()>;

#[derive(Clone)]
pub struct ExplorerDb {
    pub env: Arc<::sanakirja::Env>,
}

pub enum OpenDb {
    Initialized {
        db: ExplorerDb,
        last_stable_block: u32,
    },
    NeedsBootstrap(NeedsBootstrap),
}

pub struct NeedsBootstrap(ExplorerDb);

impl NeedsBootstrap {
    pub fn add_block0(self, block0: Block) -> Result<ExplorerDb, DbError> {
        let db = self.0;
        let mut mut_tx = db.mut_txn_begin()?;

        let parent_id = block0.parent_id();
        let block_id = block0.id();

        mut_tx.add_block0(&parent_id.into(), &block_id.into(), block0.contents.iter())?;

        mut_tx.commit()?;

        Ok(db)
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
    pub fn apply_block(&mut self, block: Block) -> Result<(), DbError> {
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
    pub fn commit(self) -> Result<(), DbError> {
        self.txn.commit()
    }
}

impl ExplorerDb {
    pub fn open<P: AsRef<Path>>(storage: Option<P>) -> Result<OpenDb, DbError> {
        let db = match storage {
            Some(path) => ExplorerDb::new(path),
            None => ExplorerDb::new_anon(),
        }?;

        let txn = db.txn_begin();

        match txn {
            Ok(txn) => Ok(OpenDb::Initialized {
                last_stable_block: txn.get_last_stable_block().get(),
                db,
            }),
            Err(DbError::UnitializedDatabase) => Ok(OpenDb::NeedsBootstrap(NeedsBootstrap(db))),
            Err(e) => Err(e),
        }
    }

    /// Try to add a new block to the indexes, this can fail if the parent of the block is not
    /// processed. This doesn't perform any validation on the given block and the previous state,
    /// it is assumed that the Block is valid
    pub async fn apply_block(&self, block: Block) -> Result<(), DbError> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || {
            let mut_tx = db.mut_txn_begin()?;

            let mut batch = Batch { txn: mut_tx };

            batch.apply_block(block)?;

            batch.commit()?;

            Ok(())
        })
        .await
        .unwrap()
    }

    pub async fn start_batch(&self) -> Result<Batch, DbError> {
        let db = self.clone();

        tokio::task::spawn_blocking(move || {
            let mut_tx = db.mut_txn_begin()?;

            Ok(Batch { txn: mut_tx })
        })
        .await
        .unwrap()
    }

    pub async fn get_txn(&self) -> Result<schema::Txn, DbError> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || {
            let txn = db.txn_begin()?;

            Ok(txn)
        })
        .await
        .unwrap()
    }

    pub async fn set_tip(&self, hash: HeaderHash) -> Result<bool, DbError> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || {
            let mut mut_tx = db.mut_txn_begin()?;

            let status = mut_tx.set_tip(&hash.into())?;

            if status {
                mut_tx.commit()?;
            }

            Ok(status)
        })
        .await
        .unwrap()
    }

    fn new<P: AsRef<Path>>(name: P) -> Result<Self, DbError> {
        Self::new_with_size(name, 1 << 20)
    }

    fn new_with_size<P: AsRef<Path>>(name: P, size: u64) -> Result<Self, DbError> {
        let env = ::sanakirja::Env::new(name, size, 2);
        match env {
            Ok(env) => Ok(Self { env: Arc::new(env) }),
            Err(e) => Err(DbError::SanakirjaError(e)),
        }
    }

    fn new_anon() -> Result<Self, DbError> {
        Self::new_anon_with_size(1 << 20)
    }

    fn new_anon_with_size(size: u64) -> Result<Self, DbError> {
        Ok(Self {
            env: Arc::new(::sanakirja::Env::new_anon(size, 2)?),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, FromBytes, AsBytes)]
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

impl From<u64> for SeqNum {
    fn from(n: u64) -> Self {
        SeqNum::new(n)
    }
}

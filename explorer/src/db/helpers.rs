use super::error::DbError;
use sanakirja::{
    btree::{self, BTreeMutPage, BTreePage},
    RootDb, Storable,
};
use std::sync::Arc;

pub(super) fn open_or_create_db<
    K: Storable,
    V: Storable,
    P: BTreePage<K, V> + BTreeMutPage<K, V>,
>(
    txn: &mut sanakirja::MutTxn<Arc<sanakirja::Env>, ()>,
    root: super::schema::Root,
) -> Result<btree::Db_<K, V, P>, DbError> {
    Ok(if let Some(db) = txn.root_db(root as usize) {
        db
    } else {
        btree::create_db_(txn)?
    })
}

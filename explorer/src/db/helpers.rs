use super::{error::DbError, Db};
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

pub(super) fn find_last_record_by<T, K, V>(
    txn: &T,
    tree: &Db<K, V>,
    key: &K,
    max_possible_value: &V,
) -> Option<V>
where
    K: Storable + PartialEq,
    V: Storable + Clone + PartialEq,
    T: ::sanakirja::LoadPage<Error = ::sanakirja::Error>,
{
    let mut cursor = btree::Cursor::new(txn, tree).unwrap();

    cursor.set(txn, key, Some(&max_possible_value)).unwrap();

    if let Some((k, _)) = cursor.prev(txn).unwrap() {
        if k == key {
            cursor.next(txn).unwrap();
        }
    }

    assert!(
        cursor
            .current(txn)
            .unwrap()
            .map(|(_, v)| v != max_possible_value)
            .unwrap_or(true),
        "ran out of sequence numbers"
    );

    cursor
        .current(txn)
        .unwrap()
        .and_then(|(k, v)| if k == key { Some(v.clone()) } else { None })
}

use super::Db;
use sanakirja::{btree, Storable};

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

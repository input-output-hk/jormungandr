use chain_core::property::{Block, BlockId, Serialize};
use chain_storage::{
    error::Error,
    store::{BackLink, BlockInfo, BlockStore},
};
use rusqlite::types::Value;
use std::path::Path;

pub struct SQLiteBlockStore<B>
where
    B: Block,
{
    pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
    dummy: std::marker::PhantomData<B>,
}

impl<B> SQLiteBlockStore<B>
where
    B: Block,
{
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let manager = r2d2_sqlite::SqliteConnectionManager::file(path);
        let pool = r2d2::Pool::new(manager).unwrap();

        let connection = pool.get().unwrap();

        connection
            .execute_batch(
                r#"
                  begin;

                  create table if not exists BlockInfo (
                    hash blob primary key,
                    depth integer not null,
                    parent blob not null,
                    fast_distance blob,
                    fast_hash blob,
                    foreign key(hash) references Blocks(hash)
                  );

                  create table if not exists Blocks (
                    hash blob primary key,
                    block blob not null
                  );

                  create table if not exists Tags (
                    name text primary key,
                    hash blob not null,
                    foreign key(hash) references BlockInfo(hash)
                  );

                  commit;
                "#,
            )
            .unwrap();

        /*
        connection
            .execute("pragma synchronous = off", rusqlite::NO_PARAMS)
            .unwrap();
        */

        connection
            .execute_batch("pragma journal_mode = WAL")
            .unwrap();

        SQLiteBlockStore {
            pool,
            dummy: std::marker::PhantomData,
        }
    }
}

fn blob_to_hash<Id: BlockId>(blob: Vec<u8>) -> Id {
    Id::deserialize(&blob[..]).unwrap()
}

impl<B> BlockStore for SQLiteBlockStore<B>
where
    B: Block,
{
    type Block = B;

    fn put_block_internal(&mut self, block: &B, block_info: BlockInfo<B::Id>) -> Result<(), Error> {
        let mut conn = self.pool.get().unwrap();

        let tx = conn.transaction().unwrap();

        tx.prepare_cached("insert into Blocks (hash, block) values(?, ?)")
            .unwrap()
            .execute(&[
                &block_info.block_hash.serialize_as_vec().unwrap()[..],
                &block.serialize_as_vec().unwrap()[..],
            ])
            .unwrap();

        let parent = block_info
            .back_links
            .iter()
            .find(|x| x.distance == 1)
            .unwrap();

        let (fast_distance, fast_hash) =
            match block_info.back_links.iter().find(|x| x.distance != 1) {
                Some(fast_link) => (
                    Value::Integer(fast_link.distance as i64),
                    Value::Blob(fast_link.block_hash.serialize_as_vec().unwrap()),
                ),
                None => (Value::Null, Value::Null),
            };

        tx
            .prepare_cached("insert into BlockInfo (hash, depth, parent, fast_distance, fast_hash) values(?, ?, ?, ?, ?)")
            .unwrap()
            .execute(&[
                Value::Blob(block_info.block_hash.serialize_as_vec().unwrap()),
                Value::Integer(block_info.depth as i64),
                Value::Blob(parent.block_hash.serialize_as_vec().unwrap()),
                fast_distance,
                fast_hash,
            ]).unwrap();

        tx.commit().unwrap();

        Ok(())
    }

    fn get_block(&self, block_hash: &B::Id) -> Result<(B, BlockInfo<B::Id>), Error> {
        let blk = self
            .pool
            .get()
            .unwrap()
            .prepare_cached("select block from Blocks where hash = ?")
            .unwrap()
            .query_row(&[&block_hash.serialize_as_vec().unwrap()[..]], |row| {
                let x: Vec<u8> = row.get(0);
                B::deserialize(&x[..]).unwrap()
            })
            .map_err(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => Error::BlockNotFound,
                _ => panic!(err),
            })?;

        let info = self.get_block_info(block_hash)?;

        Ok((blk, info))
    }

    fn get_block_info(&self, block_hash: &B::Id) -> Result<BlockInfo<B::Id>, Error> {
        self.pool
            .get()
            .unwrap()
            .prepare_cached(
                "select depth, parent, fast_distance, fast_hash from BlockInfo where hash = ?",
            )
            .unwrap()
            .query_row(&[&block_hash.serialize_as_vec().unwrap()[..]], |row| {
                let mut back_links = vec![BackLink {
                    distance: 1,
                    block_hash: blob_to_hash(row.get(1)),
                }];

                let fast_distance: Option<i64> = row.get(2);
                if let Some(fast_distance) = fast_distance {
                    back_links.push(BackLink {
                        distance: fast_distance as u64,
                        block_hash: blob_to_hash(row.get(3)),
                    });
                }

                let depth: i64 = row.get(0);

                BlockInfo {
                    block_hash: block_hash.clone(),
                    depth: depth as u64,
                    back_links,
                }
            })
            .map_err(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => Error::BlockNotFound,
                _ => panic!(err),
            })
    }

    fn put_tag(&mut self, tag_name: &str, block_hash: &B::Id) -> Result<(), Error> {
        match self
            .pool
            .get()
            .unwrap()
            .prepare_cached("insert or replace into Tags (name, hash) values(?, ?)")
            .unwrap()
            .execute(&[
                Value::Text(tag_name.to_string()),
                Value::Blob(block_hash.serialize_as_vec().unwrap()),
            ]) {
            Ok(_) => Ok(()),
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Err(Error::BlockNotFound)
            }
            Err(err) => panic!(err),
        }
    }

    fn get_tag(&self, tag_name: &str) -> Result<Option<B::Id>, Error> {
        match self
            .pool
            .get()
            .unwrap()
            .prepare_cached("select hash from Tags where name = ?")
            .unwrap()
            .query_row(&[&tag_name], |row| blob_to_hash(row.get(0)))
        {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => panic!(err),
        }
    }

    fn as_trait(&self) -> &BlockStore<Block = Self::Block> {
        self as &BlockStore<Block = Self::Block>
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use chain_storage::store::test::Block;

    #[test]
    pub fn put_get() {
        let mut store = SQLiteBlockStore::<Block>::new(":memory:");
        chain_storage::store::test::test_put_get(&mut store);
    }

    #[test]
    pub fn nth_ancestor() {
        let mut store = SQLiteBlockStore::<Block>::new(":memory:");
        chain_storage::store::test::test_nth_ancestor(&mut store);
    }

    #[test]
    pub fn iterate_range() {
        let mut store = SQLiteBlockStore::<Block>::new(":memory:");
        chain_storage::store::test::test_iterate_range(&mut store);
    }
}

extern crate chain_core;
extern crate chain_storage;
extern crate sqlite;

use chain_core::property::{Block, BlockDate, BlockId};
use chain_storage::{
    error::Error,
    store::{BackLink, BlockInfo, BlockStore},
};
use std::cell::RefCell;

pub struct SQLiteBlockStore<B>
where
    B: Block,
{
    genesis_hash: B::Id,
    connection: Box<sqlite::Connection>,

    // Prepared statements. Note: we currently give these a fake
    // static lifetime to work around the issue described in
    // https://stackoverflow.com/questions/27552670/how-to-store-sqlite-prepared-statements-for-later.
    // We use a RefCell to allow them to be used from &self methods.
    stmt_insert_block: RefCell<sqlite::Statement<'static>>,
    stmt_insert_block_info: RefCell<sqlite::Statement<'static>>,
    stmt_get_block: RefCell<sqlite::Statement<'static>>,
    stmt_get_block_info: RefCell<sqlite::Statement<'static>>,
    stmt_put_tag: RefCell<sqlite::Statement<'static>>,

    dummy: std::marker::PhantomData<B>,
    in_txn: bool,
    pending_changes: usize,
}

impl<B> SQLiteBlockStore<B>
where
    B: Block,
{
    pub fn new(genesis_hash: B::Id, path: &str) -> Self {
        let connection = Box::new(sqlite::open(path).unwrap());

        connection
            .execute(
                r#"
          create table if not exists BlockInfo (
            hash blob primary key,
            date integer not null,
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
        "#,
            )
            .unwrap();

        let make_statement = |connection: &Box<sqlite::Connection>, s: &str| unsafe {
            RefCell::new(std::mem::transmute(connection.prepare(s).unwrap()))
        };

        SQLiteBlockStore {
            genesis_hash,
            stmt_insert_block: make_statement(&connection, "insert into Blocks (hash, block) values(?, ?)"),
            stmt_insert_block_info: make_statement(&connection, "insert into BlockInfo (hash, date, depth, parent, fast_distance, fast_hash) values(?, ?, ?, ?, ?, ?)"),
            stmt_get_block: make_statement(&connection, "select block from Blocks where hash = ?"),
            stmt_get_block_info: make_statement(&connection, "select depth, parent, fast_distance, fast_hash, date from BlockInfo where hash = ?"),
            stmt_put_tag: make_statement(&connection, "insert or replace into Tags (name, hash) values(?, ?)"),
            connection,
            dummy: std::marker::PhantomData,
            in_txn: false,
            pending_changes: 0,
        }
    }

    fn flush(&mut self) {
        if self.in_txn {
            //eprintln!("flushing sqlite...");
            self.connection.execute("commit").unwrap();
            self.in_txn = false;
        }
    }

    fn do_change(&mut self) {
        if self.in_txn {
            self.pending_changes += 1;
            if self.pending_changes > 100000 {
                self.flush();
            }
        } else {
            self.connection.execute("begin transaction").unwrap();
            self.in_txn = true;
            self.pending_changes = 1;
        }
    }
}

fn blob_to_hash<Id: BlockId>(blob: Vec<u8>) -> Id {
    Id::try_from_slice(&blob[..]).unwrap()
}

impl<B> Drop for SQLiteBlockStore<B>
where
    B: Block,
{
    fn drop(&mut self) {
        self.flush();
    }
}

impl<B> BlockStore<B> for SQLiteBlockStore<B>
where
    B: Block,
{
    fn put_block_internal(
        &mut self,
        block: B,
        block_info: BlockInfo<B::Id, B::Date>,
    ) -> Result<(), Error> {
        self.do_change();

        // FIXME: wrap the next two statements in a transaction

        let mut stmt_insert_block = self.stmt_insert_block.borrow_mut();
        stmt_insert_block.reset().unwrap();
        stmt_insert_block
            .bind(1, &block_info.block_hash.as_ref()[..])
            .unwrap();
        stmt_insert_block
            .bind(2, &block.serialize_as_vec().unwrap()[..])
            .unwrap();
        stmt_insert_block.next().unwrap();

        let mut stmt_insert_block_info = self.stmt_insert_block_info.borrow_mut();
        stmt_insert_block_info.reset().unwrap();
        stmt_insert_block_info
            .bind(1, &block_info.block_hash.as_ref()[..])
            .unwrap();
        stmt_insert_block_info
            .bind(2, block_info.block_date.serialize() as i64)
            .unwrap();
        stmt_insert_block_info
            .bind(3, block_info.depth as i64)
            .unwrap();
        let parent = block_info
            .back_links
            .iter()
            .find(|x| x.distance == 1)
            .unwrap();
        stmt_insert_block_info
            .bind(4, &parent.block_hash.as_ref()[..])
            .unwrap();
        match block_info.back_links.iter().find(|x| x.distance != 1) {
            Some(fast_link) => {
                stmt_insert_block_info
                    .bind(5, fast_link.distance as i64)
                    .unwrap();
                stmt_insert_block_info
                    .bind(6, &fast_link.block_hash.as_ref()[..])
                    .unwrap();
            }
            None => {
                stmt_insert_block_info.bind(5, ()).unwrap();
                stmt_insert_block_info.bind(6, ()).unwrap();
            }
        };
        stmt_insert_block_info.next().unwrap();

        Ok(())
    }

    fn get_block(&self, block_hash: &B::Id) -> Result<(B, BlockInfo<B::Id, B::Date>), Error> {
        let mut stmt_get_block = self.stmt_get_block.borrow_mut();
        stmt_get_block.reset().unwrap();

        stmt_get_block.bind(1, &block_hash.as_ref()[..]).unwrap();

        match stmt_get_block.next().unwrap() {
            sqlite::State::Done => Err(Error::BlockNotFound),
            sqlite::State::Row => Ok((
                B::deserialize(&stmt_get_block.read::<Vec<u8>>(0).unwrap()[..]).unwrap(),
                self.get_block_info(block_hash)?,
            )),
        }
    }

    fn get_block_info(&self, block_hash: &B::Id) -> Result<BlockInfo<B::Id, B::Date>, Error> {
        let mut stmt_get_block_info = self.stmt_get_block_info.borrow_mut();
        stmt_get_block_info.reset().unwrap();

        stmt_get_block_info
            .bind(1, &block_hash.as_ref()[..])
            .unwrap();

        match stmt_get_block_info.next().unwrap() {
            sqlite::State::Done => Err(Error::BlockNotFound),
            sqlite::State::Row => {
                let mut back_links = vec![BackLink {
                    distance: 1,
                    block_hash: blob_to_hash(stmt_get_block_info.read::<Vec<u8>>(1).unwrap()),
                }];

                let fast_distance = stmt_get_block_info.read::<i64>(2).unwrap() as u64;
                if fast_distance != 0 {
                    back_links.push(BackLink {
                        distance: fast_distance,
                        block_hash: blob_to_hash(stmt_get_block_info.read::<Vec<u8>>(3).unwrap()),
                    });
                }

                Ok(BlockInfo {
                    block_hash: block_hash.clone(),
                    block_date: B::Date::deserialize(
                        stmt_get_block_info.read::<i64>(0).unwrap() as u64
                    ),
                    depth: stmt_get_block_info.read::<i64>(0).unwrap() as u64,
                    back_links,
                })
            }
        }
    }

    fn put_tag(&mut self, tag_name: &str, block_hash: &B::Id) -> Result<(), Error> {
        let mut stmt_put_tag = self.stmt_put_tag.borrow_mut();
        stmt_put_tag.reset().unwrap();
        stmt_put_tag.bind(1, tag_name).unwrap();
        stmt_put_tag.bind(2, &block_hash.as_ref()[..]).unwrap();
        stmt_put_tag.next().unwrap();
        Ok(())
    }

    fn get_tag(&self, tag_name: &str) -> Result<Option<B::Id>, Error> {
        let mut statement = self
            .connection
            .prepare("select hash from Tags where name = ?")
            .unwrap();
        statement.bind(1, tag_name).unwrap();
        match statement.next().unwrap() {
            sqlite::State::Done => Ok(None),
            sqlite::State::Row => Ok(Some(blob_to_hash(statement.read::<Vec<u8>>(0).unwrap()))),
        }
    }

    fn get_genesis_hash(&self) -> B::Id {
        self.genesis_hash.clone()
    }
}

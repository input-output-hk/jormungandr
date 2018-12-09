use super::{Hash, Block, BlockInfo, BackLink, BlockStore, Error};
use sqlite;
use cardano::util::try_from_slice::TryFromSlice;

pub struct SQLiteBlockStore<B> where B: Block {
    genesis_hash: Hash,
    connection: sqlite::Connection,
    //insert_block: sqlite::Statement<'db>,
    dummy: std::marker::PhantomData<B>,
    in_txn: bool,
    pending_changes: usize,
}

impl<B> SQLiteBlockStore<B> where B: Block {
    pub fn new(genesis_hash: Hash, path: &str) -> Self {
        let connection = sqlite::open(path).unwrap();

        connection.execute(r#"
          create table if not exists Blocks (
            hash blob primary key,
            depth integer not null,
            parent blob not null,
            fast_delta blob,
            fast_hash blob,
            block blob not null
          );

          create table if not exists Tags (
            name text primary key,
            hash blob not null,
            foreign key(hash) references Blocks(hash)
          );
        "#).unwrap();

        SQLiteBlockStore {
            genesis_hash,
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

impl<B> Drop for SQLiteBlockStore<B> where B: Block {
    fn drop(&mut self) {
        self.flush();
    }
}

impl<B> BlockStore<B> for SQLiteBlockStore<B> where B: Block {

    fn put_block_internal(&mut self, block: B, block_info: BlockInfo) -> Result<(), Error>
    {
        self.do_change();
        let mut statement = self.connection.prepare(
            "insert into Blocks (hash, depth, parent, fast_delta, fast_hash, block) values(?, ?, ?, ?, ?, ?)").unwrap();
        statement.bind(1, &block_info.block_hash[..]).unwrap();
        statement.bind(2, block_info.depth as i64).unwrap();
        let parent = block_info.back_links.iter().find(|x| x.delta == 1).unwrap();
        statement.bind(3, &parent.block_hash[..]).unwrap();
        match block_info.back_links.iter().find(|x| x.delta != 1) {
            Some(fast_link) => {
                statement.bind(4, fast_link.delta as i64).unwrap();
                statement.bind(5, &fast_link.block_hash[..]).unwrap();
            },
            None => {
                statement.bind(4, ()).unwrap();
                statement.bind(5, ()).unwrap();
            }
        };
        statement.bind(6, &block.serialize()[..]).unwrap();
        statement.next().unwrap();
        Ok(())
    }

    fn get_block(&self, block_hash: &Hash) -> Result<(B, BlockInfo), Error>
    {
        unimplemented!()
    }

    fn get_block_info(&self, block_hash: &Hash) -> Result<BlockInfo, Error>
    {
        let mut statement = self.connection.prepare(
            "select depth, parent, fast_delta, fast_hash from Blocks where hash = ?").unwrap();
        statement.bind(1, &block_hash[..]).unwrap();

        match statement.next().unwrap() {
            sqlite::State::Done =>
                Err(cardano_storage::Error::BlockNotFound(block_hash.clone().into())),
            sqlite::State::Row => {

                let mut back_links = vec![
                    BackLink {
                        delta: 1,
                        block_hash: blob_to_hash(statement.read::<Vec<u8>>(1).unwrap())
                    }
                ];

                let fast_delta = statement.read::<i64>(2).unwrap() as u64;
                if fast_delta != 0 {
                    back_links.push(BackLink {
                        delta: fast_delta,
                        block_hash: blob_to_hash(statement.read::<Vec<u8>>(3).unwrap())
                    });
                }

                Ok(BlockInfo {
                    block_hash: block_hash.clone(),
                    depth: statement.read::<i64>(0).unwrap() as u64,
                    back_links
                })
            }
        }
    }

    fn put_tag(&mut self, tag_name: &str, block_hash: &Hash) -> Result<(), Error>
    {
        let mut statement = self.connection.prepare(
            "insert or replace into Tags (name, hash) values(?, ?)").unwrap();
        statement.bind(1, tag_name).unwrap();
        statement.bind(2, &block_hash[..]).unwrap();
        statement.next().unwrap();
        Ok(())
    }

    fn get_tag(&self, tag_name: &str) -> Result<Option<Hash>, Error>
    {
        let mut statement = self.connection.prepare(
            "select hash from Tags where name = ?").unwrap();
        statement.bind(1, tag_name).unwrap();
        match statement.next().unwrap() {
            sqlite::State::Done => Ok(None),
            sqlite::State::Row =>
                Ok(Some(blob_to_hash(statement.read::<Vec<u8>>(0).unwrap())))
        }
    }

    fn get_genesis_hash(&self) -> Hash {
        self.genesis_hash.clone()
    }
}

fn blob_to_hash(blob: Vec<u8>) -> Hash {
    Hash::try_from_slice(&blob[..]).unwrap()
}

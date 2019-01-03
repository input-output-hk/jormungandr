extern crate jormungandr;
extern crate cardano_storage;

use cardano_storage::{StorageConfig};
use jormungandr::storage::{self, BlockStore, ChainStateStore};
use jormungandr::blockchain::{Block, ChainState};
use std::time::{Instant};
use std::str::FromStr;

fn main() {
    let storage_config = StorageConfig::new(&"/home/eelco/.local/share/cardano-cli/blockchains/staging".into());
    let storage = cardano_storage::Storage::init(&storage_config).unwrap();

    let genesis_hash = cardano::block::HeaderHash::from_str(&"c6a004d3d178f600cd8caa10abbebe1549bef878f0665aea2903472d5abf7323").unwrap();

    let first_hash = cardano::block::HeaderHash::from_str(&"b365f1be6863b453f12b93e1810909b10c79a95ee44bf53414888513fe172c90").unwrap();

    let tip_hash = cardano::block::HeaderHash::from_str(&"159e3cfe147dfbe302daf37b63dbce2e676cd88c1419f693ad9ebdc69cf4bc1c").unwrap();
    //let tip_hash = cardano::block::HeaderHash::from_str(&"9fded76b6d9d05aa4bd3f2f3d5219e4ad21ef19997914b94e2ac08064e94724a").unwrap();

    let mut store = storage::memory::MemoryBlockStore::<cardano::block::ChainState>
        ::new(&exe_common::parse_genesis_data::parse_genesis_data(
            exe_common::genesis_data::get_genesis_data(&genesis_hash).unwrap().as_bytes()));
    //let mut store = storage::sqlite::SQLiteBlockStore::new((*genesis_hash).into(), "/tmp/chain.sqlite");

    /* Convert a chain using old-school storage to a MemoryBlockStore. */
    let now = Instant::now();
    let mut chain_state = store.get_chain_state_at(&store.get_genesis_hash()).unwrap();
    for (n, res) in cardano_storage::iter::Iter::new(&storage, first_hash.clone(), tip_hash.clone()).unwrap().enumerate() {
        let (_raw_blk, blk) = res.unwrap();
        let hash = blk.get_hash();
        chain_state.apply_block(&blk).unwrap();
        store.put_block(blk).unwrap();
        store.put_chain_state(&chain_state).unwrap();
        //if n > 49900 { break; }
        if n % 10000 == 0 {
            println!(".");
        }
        store.put_tag(&"tip", &hash).unwrap();
        assert_eq!(store.get_tag(&"tip").unwrap().unwrap(), hash);
    }

    let d = now.elapsed();
    println!("load duration {}", d.as_secs() as u64 * 1000 + d.subsec_millis() as u64);

    let tip_hash = store.get_tag(&"tip").unwrap().unwrap();

    let tip_info = store.get_block_info(&(*tip_hash).into()).unwrap();
    println!("hash = {}, chain length = {}", cardano::block::HeaderHash::from(tip_info.block_hash), tip_info.depth);
    //assert_eq!(tip_info.depth, 1874655);

    let delta = 12345;

    let block_info2 = store.get_nth_ancestor(&(*tip_hash).into(), delta).unwrap();
    println!("hash = {}, chain length = {}", cardano::block::HeaderHash::from(block_info2.block_hash), block_info2.depth);
    assert_eq!(tip_info.depth, block_info2.depth + delta);

    assert_eq!(store.is_ancestor(&block_info2.block_hash, &block_info2.block_hash).unwrap(), Some(0));
    assert_eq!(store.is_ancestor(&block_info2.block_hash, &tip_info.block_hash).unwrap(), Some(delta));

    let mut n = 0;
    for info in store.iterate_range(&block_info2.block_hash, &tip_info.block_hash).unwrap() {
        //let info = info.unwrap();
        //println!("block {} {}", info.block_hash, info.depth);
        n += 1;
    }
    assert_eq!(n, delta);

    let now = Instant::now();
    let mut n = 0;
    for info in store.iterate_range(&store.get_genesis_hash(), &tip_info.block_hash).unwrap() {
        n += 1;
    }
    let d = now.elapsed();
    println!("hash iteration duration {}", d.as_secs() as u64 * 1000 + d.subsec_millis() as u64);
    assert_eq!(n, tip_info.depth);

    let now = Instant::now();
    let mut n = 0;
    for info in store.iterate_range(&store.get_genesis_hash(), &tip_info.block_hash).unwrap() {
        store.get_block(&info.unwrap().block_hash).unwrap();
        n += 1;
    }
    let d = now.elapsed();
    println!("block iteration duration {}", d.as_secs() as u64 * 1000 + d.subsec_millis() as u64);
    assert_eq!(n, tip_info.depth);

    // Validate the chain.
    let now = Instant::now();
    let chain_state = store.get_chain_state_at(&tip_info.block_hash).unwrap();
    let d = now.elapsed();
    println!("validation duration {}", d.as_secs() as u64 * 1000 + d.subsec_millis() as u64);
    assert_eq!(chain_state.get_last_block(), tip_info.block_hash);

    /*
    // Validate again. This should be much faster because of chain
    // state serialization.
    let now = Instant::now();
    let chain_state = store.get_chain_state_at(&tip_info.block_hash).unwrap();
    let d = now.elapsed();
    println!("validation duration {}", d.as_secs() as u64 * 1000 + d.subsec_millis() as u64);
    assert_eq!(chain_state.get_last_block(), tip_info.block_hash);
    */
}

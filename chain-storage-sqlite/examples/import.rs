extern crate cardano;
extern crate cardano_storage;
extern crate chain_storage;

use cardano_storage::StorageConfig;
use chain_core::property::Block;
use chain_storage::store::BlockStore;
use std::env;
use std::str::FromStr;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();
    let storage_path = &args[1];
    let db_path = &args[2];

    let storage_config = StorageConfig::new(&storage_path.clone().into());
    let storage = cardano_storage::Storage::init(&storage_config).unwrap();

    let genesis_hash = cardano::block::HeaderHash::from_str(
        &"c6a004d3d178f600cd8caa10abbebe1549bef878f0665aea2903472d5abf7323",
    )
    .unwrap();

    let first_hash = cardano::block::HeaderHash::from_str(
        &"b365f1be6863b453f12b93e1810909b10c79a95ee44bf53414888513fe172c90",
    )
    .unwrap();

    let tip_hash = cardano::block::HeaderHash::from_str(
        &"159e3cfe147dfbe302daf37b63dbce2e676cd88c1419f693ad9ebdc69cf4bc1c",
    )
    .unwrap();

    let mut chain_state = cardano::block::ChainState::new(&exe_common::genesisdata::parse::parse(
        exe_common::genesis_data::get_genesis_data(&genesis_hash).unwrap().as_bytes()));

    let mut store = chain_storage_sqlite::SQLiteBlockStore::new(genesis_hash, db_path);

    /* Convert a chain using old-school storage to a SQLiteBlockStore. */
    let now = Instant::now();
    let mut last_hash = None;
    for (n, res) in cardano_storage::iter::Iter::new(&storage, first_hash.clone(), tip_hash.clone())
        .unwrap()
        .enumerate()
    {
        let (_raw_blk, blk) = res.unwrap();
        let hash = blk.id();
        chain_state.verify_block(&hash, &blk).unwrap();
        store.put_block(blk).unwrap();
        //if n > 49900 { break; }
        if n % 10000 == 0 {
            println!(".");
        }
        /*
        store.put_tag(&"tip", &hash).unwrap();
        assert_eq!(store.get_tag(&"tip").unwrap().unwrap(), hash);
         */
        last_hash = Some(hash);
    }

    let last_hash = last_hash.unwrap();
    store.put_tag(&"tip", &last_hash).unwrap();
    assert_eq!(store.get_tag(&"tip").unwrap().unwrap(), last_hash);

    let d = now.elapsed();
    println!(
        "load duration {}",
        d.as_secs() as u64 * 1000 + d.subsec_millis() as u64
    );

    let tip_hash = store.get_tag(&"tip").unwrap().unwrap();

    let tip_info = store.get_block_info(&tip_hash).unwrap();
    println!(
        "hash = {}, chain length = {}",
        tip_info.block_hash, tip_info.depth
    );

    let delta = 12345;

    let block_info2 = store.get_nth_ancestor(&tip_hash, delta).unwrap();
    println!(
        "hash = {}, chain length = {}",
        block_info2.block_hash, block_info2.depth
    );
    assert_eq!(tip_info.depth, block_info2.depth + delta);

    assert_eq!(
        store
            .is_ancestor(&block_info2.block_hash, &block_info2.block_hash)
            .unwrap(),
        Some(0)
    );
    assert_eq!(
        store
            .is_ancestor(&block_info2.block_hash, &tip_info.block_hash)
            .unwrap(),
        Some(delta)
    );

    let mut n = block_info2.depth;
    for info in store
        .iterate_range(&block_info2.block_hash, &tip_info.block_hash)
        .unwrap()
    {
        let info = info.unwrap();
        n += 1;
        assert_eq!(info.depth, n);
    }
    assert_eq!(n, tip_info.depth);

    let now = Instant::now();
    let mut n = 0;
    for info in store
        .iterate_range(&store.get_genesis_hash(), &tip_info.block_hash)
        .unwrap()
    {
        n += 1;
        assert_eq!(info.unwrap().depth, n);
    }
    let d = now.elapsed();
    println!(
        "hash iteration duration {}",
        d.as_secs() as u64 * 1000 + d.subsec_millis() as u64
    );
    assert_eq!(n, tip_info.depth);

    let now = Instant::now();
    let mut n = 0;
    for info in store
        .iterate_range(&store.get_genesis_hash(), &tip_info.block_hash)
        .unwrap()
    {
        let info = info.unwrap();
        store.get_block(&info.block_hash).unwrap();
        n += 1;
        assert_eq!(info.depth, n);
    }
    let d = now.elapsed();
    println!(
        "block iteration duration {}",
        d.as_secs() as u64 * 1000 + d.subsec_millis() as u64
    );
    assert_eq!(n, tip_info.depth);
}

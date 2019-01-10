extern crate cardano;
extern crate cardano_storage;
extern crate chain_storage;

use cardano_storage::StorageConfig;
use chain_core::property::{Block, ChainState};
use chain_storage::store::{BlockStore, ChainStateStore};
use std::env;
use std::str::FromStr;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();
    let storage_path = &args[1];

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

    let mut store = chain_storage::memory::MemoryBlockStore::<cardano::block::ChainState>::new(
        &exe_common::parse_genesis_data::parse_genesis_data(
            exe_common::genesis_data::get_genesis_data(&genesis_hash)
                .unwrap()
                .as_bytes(),
        ),
    );

    /* Convert a chain using old-school storage to a MemoryBlockStore. */
    let now = Instant::now();
    let mut chain_state = store.get_chain_state_at(&store.get_genesis_hash()).unwrap();
    let mut last_hash = None;
    for (n, res) in cardano_storage::iter::Iter::new(&storage, first_hash.clone(), tip_hash.clone())
        .unwrap()
        .enumerate()
    {
        let (_raw_blk, blk) = res.unwrap();
        let hash = blk.id();
        chain_state.apply_block(&blk).unwrap();
        store.put_block(blk).unwrap();
        store.put_chain_state(&chain_state).unwrap();
        //if n > 49900 { break; }
        if n % 10000 == 0 {
            println!(".");
        }
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

    // Restore chain state.
    let now = Instant::now();
    let chain_state = store.get_chain_state_at(&tip_hash).unwrap();
    let d = now.elapsed();
    println!(
        "restore duration {}",
        d.as_secs() as u64 * 1000 + d.subsec_millis() as u64
    );
    assert_eq!(chain_state.get_last_block_id(), tip_hash);
}

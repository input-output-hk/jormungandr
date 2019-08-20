#[macro_use(lazy_static)]
extern crate lazy_static;
#[macro_use(error_chain, bail)]
extern crate error_chain;

mod programs;
#[macro_use]
pub mod scenario;
pub mod node;
mod slog;
mod wallet;

pub use self::node::Node;
pub use self::programs::{JCLI, JORMUNGANDR};
pub use self::scenario::{NodeAlias, WalletAlias, WalletType};
pub use self::slog::{Error as SlogCodecError, SlogCodec};
pub use self::wallet::Wallet;

#[test]
fn scenario_1() {
    let mut context = scenario::Context::new();

    let mut scenario = prepare_scenario! {
        &mut context,
        topology [
            "node1",
            "node2" -> "node1",
        ]
        blockchain {
            consensus = Bft,
            number_of_slots_per_epoch = 10,
            slot_duration = 1,
            leaders = [ "node1", "node2" ],
            initials = [
                account "faucet1" with 1_000_000_000,
                account "faucet2" with 2_000_000_000 delegates to "node2",
            ],
        }
    }
    .unwrap();

    scenario.spawn_node("node1", true).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    scenario.spawn_node("node2", false).unwrap();

    std::thread::sleep(std::time::Duration::from_secs(20));

    let node1_tip_hash = scenario.get_tip("node1").unwrap();
    let node2_tip_hash = scenario.get_tip("node2").unwrap();
    println!("got tip from node 1: {}", node1_tip_hash);
    println!("got tip from node 2: {}", node2_tip_hash);

    std::thread::sleep(std::time::Duration::from_secs(1));
    let node1_block = scenario.get_block("node1", &node2_tip_hash).unwrap();
    println!("got block {} from node2", node1_tip_hash);
    let node2_block = scenario.get_block("node2", &node1_tip_hash).unwrap();
    println!("got block {} from node2", node1_tip_hash);

    dbg!(&node1_block);
    dbg!(&node2_block);

    assert_eq!(node1_tip_hash, node2_tip_hash);
}

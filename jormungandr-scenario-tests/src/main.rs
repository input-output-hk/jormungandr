#[macro_use]
extern crate jormungandr_scenario_tests;

use jormungandr_scenario_tests::{prepare_command, Context, Seed};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
struct CommandArgs {
    /// path or name of the jormungandr node to test
    #[structopt(long = "jormungandr", default_value = "jormungandr")]
    jormungandr: PathBuf,

    /// path or name of the jcli to test
    #[structopt(long = "jcli", default_value = "jcli")]
    jcli: PathBuf,

    /// to set if to reproduce an existing test
    #[structopt(long = "seed")]
    seed: Option<Seed>,
}

fn main() {
    let command_args = CommandArgs::from_args();

    let jormungandr = prepare_command(command_args.jormungandr);
    let jcli = prepare_command(command_args.jcli);
    let seed = command_args
        .seed
        .unwrap_or_else(|| Seed::generate(rand::rngs::OsRng::new().unwrap()));

    let mut context = Context::new(seed, jormungandr, jcli);

    scenario_1(context.derive());
}

use rand_chacha::ChaChaRng;

pub fn scenario_1(mut context: Context<ChaChaRng>) {
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

    scenario.spawn_node(&context, "node1", true).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    scenario.spawn_node(&context, "node2", false).unwrap();

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

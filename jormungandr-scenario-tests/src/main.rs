#[macro_use]
extern crate jormungandr_scenario_tests;

#[macro_use]
extern crate jormungandr_integration_tests;

use jormungandr_scenario_tests::{
    node::{LeadershipMode, PersistenceMode},
    prepare_command, style, Context, Seed,
};
use std::{collections::HashMap, path::PathBuf, thread, time::Duration};
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

    #[structopt(long = "scenario", default_value = "scenario_1")]
    scenario: String,
    /// set a directory in which the tests will be run, allowing every details
    /// to be save persistently. By default it will create temporary directories
    /// and will delete the files and documents
    #[structopt(long = "root-dir")]
    testing_directory: Option<PathBuf>,

    /// document the different scenario, creating markdown and dot (graphviz) files
    /// describing the tests initial setup
    ///
    /// The files are created within the `--root-dir`  directory.
    #[structopt(long = "document")]
    generate_documentation: bool,

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
    let testing_directory = command_args.testing_directory;
    let generate_documentation = command_args.generate_documentation;

    let mut context = Context::new(
        seed,
        jormungandr,
        jcli,
        testing_directory,
        generate_documentation,
    );

    introduction(&context);
    match should_run_all(&command_args.scenario) {
        true => run_all_scenarios(&mut context),
        false => run_scenario_by_name(&command_args.scenario, &mut context),
    }
}

fn should_run_all(scenario: &str) -> bool {
    scenario.trim() == "*"
}

pub fn run_all_scenarios(mut context: &mut Context<ChaChaRng>) {
    for scenario_to_run in scenarios_repository().values() {
        scenario_to_run(context.derive())
    }
}

pub fn run_scenario_by_name(scenario: &str, mut context: &mut Context<ChaChaRng>) {
    let repo = scenarios_repository();
    if !repo.contains_key(scenario) {
        panic!(format!(
            "Cannot find scenario '{}'. Available are: {:?}",
            scenario,
            repo.keys().cloned().collect::<Vec<String>>()
        ));
    }

    let scenario_to_run = repo.get(scenario).unwrap();
    scenario_to_run(context.derive());
}

type ScenarioMethod = fn(Context<ChaChaRng>) -> ();

pub fn scenarios_repository() -> HashMap<String, ScenarioMethod> {
    let mut map: HashMap<String, ScenarioMethod> = HashMap::new();
    map.insert(
        "two_transaction_to_two_leaders".to_string(),
        two_transaction_to_two_leaders,
    );
    map.insert("transaction_to_passive".to_string(), transaction_to_passive);
    map.insert("leader_is_offline".to_string(), leader_is_offline);
    map.insert(
        "leader_is_online_with_delay".to_string(),
        leader_is_online_with_delay,
    );
    map.insert("leader_restart".to_string(), leader_restart);
    map.insert(
        "passive_node_is_updated".to_string(),
        passive_node_is_updated,
    );
    map.insert("star".to_string(), star);
    map.insert("ring".to_string(), ring);
    map.insert("mesh".to_string(), mesh);
    map.insert("point_to_point".to_string(), point_to_point);
    map.insert("tree".to_string(), tree);
    map.insert("relay".to_string(), relay);
    map.insert("scenario_1".to_string(), scenario_1);
    map.insert("scenario_2".to_string(), scenario_2);
    map
}

fn introduction<R: rand_core::RngCore>(context: &Context<R>) {
    println!(
        r###"
        ---_ ......._-_--.
       (|\ /      / /| \  \               _  ___  ____  __  __ _   _ _   _  ____    _    _   _ ____  ____
       /  /     .'  -=-'   `.            | |/ _ \|  _ \|  \/  | | | | \ | |/ ___|  / \  | \ | |  _ \|  _ \
      /  /    .'             )        _  | | | | | |_) | |\/| | | | |  \| | |  _  / _ \ |  \| | | | | |_) |
    _/  /   .'        _.)   /        | |_| | |_| |  _ <| |  | | |_| | |\  | |_| |/ ___ \| |\  | |_| |  _ <
   /   o  o       _.-' /  .'          \___/ \___/|_| \_\_|  |_|\___/|_| \_|\____/_/   \_\_| \_|____/|_| \_\
   \          _.-'    / .'#|
    \______.-'//    .'.' \#|         SCENARIO TEST SUITE
     \|  \ | //   .'.' _ |#|
      `   \|//  .'.'_._._|#|
       .  .// .'.' | _._ \#|
       \`-|\_/ /    \ _._ \#\
        `/'\__/      \ _._ \#\
       /^|            \ _-_ \#
      '  `             \ _-_ \
                        \_

 {}jormungandr: {}
 {}jcli:        {}
 {}seed:        {}

###############################################################################
    "###,
        *style::icons::jormungandr,
        style::binary.apply_to(context.jormungandr().to_string()),
        *style::icons::jcli,
        style::binary.apply_to(context.jcli().to_string()),
        *style::icons::seed,
        style::seed.apply_to(context.seed()),
    )
}

use rand_chacha::ChaChaRng;

pub fn scenario_1(mut context: Context<ChaChaRng>) {
    let scenario_settings = prepare_scenario! {
        "simple network example",
        &mut context,
        topology [
            "node1",
            "node2" -> "node1",
        ]
        blockchain {
            consensus = Bft,
            number_of_slots_per_epoch = 10,
            slot_duration = 1,
            leaders = [ "node1" ],
            initials = [
                account "faucet1" with 1_000_000_000,
                account "faucet2" with 2_000_000_000 delegates to "node2",
            ],
        }
    };

    let mut controller = scenario_settings.build(context).unwrap();

    let node1 = controller
        .spawn_node("node1", LeadershipMode::Leader, PersistenceMode::InMemory)
        .unwrap();
    let node2 = controller
        .spawn_node("node2", LeadershipMode::Passive, PersistenceMode::InMemory)
        .unwrap();

    controller.monitor_nodes();
    std::thread::sleep(std::time::Duration::from_secs(10));
    let tip1 = node1.tip().unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    node1.shutdown().unwrap();
    let _block = node2.block(&tip1).unwrap();

    std::thread::sleep(std::time::Duration::from_secs(1));

    node2.shutdown().unwrap();

    controller.finalize();
}

pub fn scenario_2(mut context: Context<ChaChaRng>) {
    let scenario_settings = prepare_scenario! {
        "Testing the network",
        &mut context,
        topology [
            "Leader1",
            "Passive1" -> "Leader1",
            "Passive2" -> "Leader1",
            "Passive3" -> "Leader1",
            "Unknown1",
        ]
        blockchain {
            consensus = GenesisPraos,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ "Leader2" ],
            initials = [
                account "unassigned1" with   500_000_000,
                account "unassigned2" with   100_000_000,
                account "delegated1" with  2_000_000_000 delegates to "Leader1",
                account "delegated2" with    300_000_000 delegates to "Unknown1",
            ],
        }
    };

    let mut controller = scenario_settings.build(context).unwrap();

    let leader1 = controller
        .spawn_node("Leader1", LeadershipMode::Leader, PersistenceMode::InMemory)
        .unwrap();
    thread::sleep(Duration::from_secs(2));
    let passive1 = controller
        .spawn_node(
            "Passive1",
            LeadershipMode::Passive,
            PersistenceMode::InMemory,
        )
        .unwrap();
    let _passive2 = controller
        .spawn_node(
            "Passive2",
            LeadershipMode::Passive,
            PersistenceMode::InMemory,
        )
        .unwrap();
    let _passive3 = controller
        .spawn_node(
            "Passive3",
            LeadershipMode::Passive,
            PersistenceMode::InMemory,
        )
        .unwrap();

    controller.monitor_nodes();

    let mut wallet1 = controller.wallet("unassigned1").unwrap();
    let wallet2 = controller.wallet("delegated1").unwrap();

    loop {
        let check = controller
            .wallet_send_to(&mut wallet1, &wallet2, &leader1, 5_000.into())
            .unwrap();

        thread::sleep(Duration::from_secs(1));

        let status = leader1.wait_fragment(Duration::from_secs(2), check);

        if let Ok(status) = status {
            if status.is_in_a_block() {
                wallet1.confirm_transaction();
            } else {
                break;
            }
        } else {
            break;
        }
    }

    leader1.shutdown().unwrap();
    passive1.shutdown().unwrap();

    controller.finalize();
}

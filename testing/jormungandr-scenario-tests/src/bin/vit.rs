use jormungandr_scenario_tests::{
    node::{LeadershipMode, PersistenceMode},
    programs::prepare_command,
    scenario::{
        parse_progress_bar_mode_from_str, repository::ScenarioResult, Context, ProgressBarMode,
        Seed,
    },
    test::Result,
};

use jormungandr_scenario_tests::prepare_scenario;

use function_name::named;
use jormungandr_lib::interfaces::Explorer;
use jormungandr_testing_utils::testing::network_builder::SpawnParams;
use rand_chacha::ChaChaRng;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(setting = structopt::clap::AppSettings::ColoredHelp)]
struct CommandArgs {
    /// path or name of the jormungandr node to test
    #[structopt(long = "jormungandr", default_value = "jormungandr")]
    jormungandr: PathBuf,

    /// path or name of the jcli to test
    #[structopt(long = "jcli", default_value = "jcli")]
    jcli: PathBuf,

    /// set a directory in which the tests will be run, allowing every details
    /// to be save persistently. By default it will create temporary directories
    /// and will delete the files and documents
    #[structopt(long = "root-dir")]
    testing_directory: PathBuf,

    /// in some circumstances progress bar can spoil test logs (e.g. on test build job)
    /// if this parametrer value is true, then no progress bar is visible,
    /// but simple log on console enabled
    ///
    /// no progress bar, only simple console output
    #[structopt(
        long = "progress-bar-mode",
        default_value = "Monitor",
        parse(from_str = parse_progress_bar_mode_from_str)
    )]
    progress_bar_mode: ProgressBarMode,

    /// to set if to reproduce an existing test
    #[structopt(long = "seed")]
    seed: Option<Seed>,

    /// level for all nodes
    #[structopt(long = "log-level", default_value = "info")]
    log_level: String,
}

fn main() -> Result<()> {
    let command_args = CommandArgs::from_args();

    std::env::set_var("RUST_BACKTRACE", "full");

    let jormungandr = prepare_command(&command_args.jormungandr);
    let jcli = prepare_command(&command_args.jcli);
    let progress_bar_mode = command_args.progress_bar_mode;
    let seed = command_args
        .seed
        .unwrap_or_else(|| Seed::generate(rand::rngs::OsRng));
    let testing_directory = command_args.testing_directory;
    let generate_documentation = true;
    let log_level = command_args.log_level;

    let context = Context::new(
        seed,
        jormungandr,
        jcli,
        Some(testing_directory),
        generate_documentation,
        progress_bar_mode,
        log_level,
    );

    jormungandr_scenario_tests::introduction::print(&context, "VOTING BACKEND");
    vote_backend(context).map(|_| ())
}

const LEADER_1: &str = "Leader1";
const LEADER_2: &str = "Leader2";
const LEADER_3: &str = "Leader3";
const LEADER_4: &str = "Leader4";
const WALLET_NODE: &str = "Wallet_Node";

#[named]
#[allow(unreachable_code)]
#[allow(clippy::empty_loop)]
pub fn vote_backend(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let _name = function_name!();
    let scenario_settings = prepare_scenario! {
        "vote_backend",
        &mut context,
        topology [
            LEADER_1,
            LEADER_2 -> LEADER_1,
            LEADER_3 -> LEADER_1,
            LEADER_4 -> LEADER_1,
            WALLET_NODE -> LEADER_1,LEADER_2,LEADER_3,LEADER_4
        ]
        blockchain {
            consensus = Bft,
            number_of_slots_per_epoch = 60,
            slot_duration = 10,
            leaders = [ LEADER_1, LEADER_2, LEADER_3, LEADER_4 ],
            initials = [
                "account" "Committee" with 500_000_000,
                "utxo" "Alice" with 100_000,
                "utxo" "Bob" with 100_000,
            ],
            committees = [ "Committee" ],
            vote_plans = [
                "fund1" from "Committee" through epochs 0->1->2 contains proposals = [
                    proposal adds 1_000_000 to "rewards" with 3 vote options,
                ]
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    // bootstrap network
    let leader_1 = controller.spawn_node_custom(
        SpawnParams::new(LEADER_1)
            .leader()
            .persistence_mode(PersistenceMode::Persistent)
            .explorer(Explorer { enabled: true }),
    )?;
    leader_1.wait_for_bootstrap()?;
    controller.monitor_nodes();

    //start bft node 2
    let leader_2 = controller.spawn_node(
        LEADER_2,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader_2.wait_for_bootstrap()?;

    //start bft node 3
    let leader_3 = controller.spawn_node(
        LEADER_3,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader_3.wait_for_bootstrap()?;

    //start bft node 4
    let leader_4 = controller.spawn_node(
        LEADER_4,
        LeadershipMode::Leader,
        PersistenceMode::Persistent,
    )?;
    leader_4.wait_for_bootstrap()?;

    // start passive node
    let wallet_node = controller.spawn_node_custom(
        SpawnParams::new(WALLET_NODE)
            .passive()
            .persistence_mode(PersistenceMode::Persistent)
            .explorer(Explorer { enabled: true }),
    )?;
    wallet_node.wait_for_bootstrap()?;

    // start proxy and vit station
    let _vit_station = controller.spawn_vit_station()?;
    let _wallet_proxy = controller.spawn_wallet_proxy(WALLET_NODE)?;

    loop {}
    Ok(ScenarioResult::passed(_name))
}

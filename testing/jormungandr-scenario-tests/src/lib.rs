#[macro_use(error_chain, bail)]
extern crate error_chain;

pub mod introduction;
pub mod legacy;
pub mod node;
pub mod programs;
#[macro_use]
pub mod scenario;
pub mod example_scenarios;
pub mod interactive;
pub mod report;
pub mod test;

pub use jortestkit::console::style;
pub use node::{Node, NodeBlock0, NodeController};
pub use programs::prepare_command;
pub use scenario::{
    parse_progress_bar_mode_from_str,
    repository::{parse_tag_from_str, ScenarioResult, Tag},
    Context, ProgressBarMode, Seed,
};

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

    /// scenario name
    #[structopt(long = "scenario", default_value = "*")]
    scenario: String,

    /// in order to group scenarios (for example long_running, short running)
    /// one can use tag parameter to run entire set of scenarios
    /// by default all scenarios are run
    #[structopt(
        long = "tag",
        default_value = "All",
        parse(try_from_str = parse_tag_from_str)
    )]
    tag: Tag,

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

    /// set exit code based on test result
    #[structopt(long = "set-exit-code")]
    set_exit_code: bool,

    /// to set if to reproduce an existing test
    #[structopt(long = "seed")]
    seed: Option<Seed>,

    /// level for all nodes
    #[structopt(long = "log-level", default_value = "info")]
    log_level: String,

    /// report statuses for all unstable tests
    #[structopt(long = "report-unstable")]
    report_unstable: bool,

    /// does not silence panics in tests
    #[structopt(long = "print_panics")]
    print_panics: bool,

    /// lists tests under tag
    #[structopt(long = "list-only")]
    list_only: Option<String>,

    /// print junit like report to output
    #[structopt(short = "r", long = "print-report")]
    report: bool,
}

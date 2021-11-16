use jormungandr_scenario_tests::{
    programs::prepare_command,
    report::Reporter,
    scenario::{
        parse_progress_bar_mode_from_str,
        repository::{parse_tag_from_str, ScenariosRepository, Tag},
        Context, ProgressBarMode,
    },
};
pub use jortestkit::console::style;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(setting = structopt::clap::AppSettings::ColoredHelp)]
struct CommandArgs {
    /// path or name of the jormungandr node to test
    #[structopt(long = "jormungandr", default_value = "jormungandr")]
    jormungandr: PathBuf,

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

fn main() {
    let command_args = CommandArgs::from_args();

    std::env::set_var("RUST_BACKTRACE", "full");

    let jormungandr = prepare_command(&command_args.jormungandr);
    let progress_bar_mode = command_args.progress_bar_mode;
    let testing_directory = command_args.testing_directory;
    let generate_documentation = command_args.generate_documentation;
    let log_level = command_args.log_level;

    let context = Context::new(
        jormungandr,
        testing_directory,
        generate_documentation,
        progress_bar_mode,
        log_level,
    );

    jormungandr_scenario_tests::introduction::print(&context, "SCENARIO TEST SUITE");
    let scenarios_repo = ScenariosRepository::new(
        command_args.scenario,
        command_args.tag,
        command_args.report_unstable,
        command_args.print_panics,
    );

    if let Some(tag_to_list) = command_args.list_only {
        println!("Scenarios under tag: {}", tag_to_list.to_uppercase());
        println!(
            "{:#?}",
            scenarios_repo
                .scenarios_tagged_by(parse_tag_from_str(&tag_to_list).unwrap())
                .iter()
                .map(|sc| sc.name())
                .collect::<Vec<String>>()
        );
        std::process::exit(0);
    }

    let scenario_suite_result = scenarios_repo.run(&context);
    println!("{}", scenario_suite_result.result_string());

    if command_args.report {
        let reporter = Reporter::new(scenario_suite_result.clone());
        reporter.print()
    }

    if command_args.set_exit_code {
        std::process::exit(if scenario_suite_result.is_failed() {
            1
        } else {
            0
        });
    }
}

use jormungandr_scenario_tests::{
    programs::prepare_command,
    scenario::{parse_progress_bar_mode_from_str, Context, ProgressBarMode},
};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(setting = structopt::clap::AppSettings::ColoredHelp)]
struct CommandArgs {
    /// path or name of the jormungandr node to test
    #[structopt(long = "jormungandr", default_value = "jormungandr")]
    jormungandr: PathBuf,

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
        default_value = "None",
        parse(from_str = parse_progress_bar_mode_from_str)
    )]
    progress_bar_mode: ProgressBarMode,

    /// level for all nodes
    #[structopt(long = "log-level", default_value = "info")]
    log_level: String,
}

fn main() -> Result<(), jormungandr_scenario_tests::test::Error> {
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

    jormungandr_scenario_tests::introduction::print(&context, "INTERACTIVE SCENARIO");
    jormungandr_scenario_tests::controller::interactive_scenario(context).map(|_| ())
}

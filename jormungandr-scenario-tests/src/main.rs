use jormungandr_scenario_tests::{prepare_command, scenario_1, Context};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct CommandArgs {
    #[structopt(long = "jormungandr", default_value = "jormungandr")]
    jormungandr: PathBuf,

    #[structopt(long = "jcli", default_value = "jcli")]
    jcli: PathBuf,
}

fn main() {
    let command_args = CommandArgs::from_args();

    let jormungandr = prepare_command(command_args.jormungandr);
    let jcli = prepare_command(command_args.jcli);

    let context = Context::new(jormungandr, jcli);

    scenario_1(context);
}

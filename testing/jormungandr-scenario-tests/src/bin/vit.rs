use jormungandr_scenario_tests::{test::Result, vit::args::VitCliCommand};
use structopt::StructOpt;

fn main() -> Result<()> {
    VitCliCommand::from_args().exec()
}

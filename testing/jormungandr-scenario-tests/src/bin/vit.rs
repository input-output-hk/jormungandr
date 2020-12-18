use jormungandr_scenario_tests::{
    vit::args::VitCliCommand,
    test::Result
};
use structopt::StructOpt;

fn main() -> Result<()> {
    VitCliCommand::from_args().exec()
}

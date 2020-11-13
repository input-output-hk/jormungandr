use jormungandr_integration_tests::common::load::{FragmentLoadCommand, FragmentLoadCommandError};
use structopt::StructOpt;

pub fn main() -> Result<(), FragmentLoadCommandError> {
    FragmentLoadCommand::from_args().exec()
}

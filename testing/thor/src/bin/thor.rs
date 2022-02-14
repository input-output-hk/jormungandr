mod cli;

use thor::cli::CliController;
use cli::command::IapyxCommand;
use structopt::StructOpt;

pub fn main() {
    let controller = CliController::new().unwrap();
    IapyxCommand::from_args().exec(controller).unwrap();
}

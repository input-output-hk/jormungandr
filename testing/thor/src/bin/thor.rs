mod cli;

use cli::command::IapyxCommand;
use structopt::StructOpt;
use thor::cli::CliController;

pub fn main() {
    let controller = CliController::new().unwrap();
    IapyxCommand::from_args().exec(controller).unwrap();
}

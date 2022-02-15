mod cli;

use cli::command::Command;
use structopt::StructOpt;
use thor::cli::CliController;

pub fn main() {
    let controller = CliController::new().unwrap();
    Command::from_args().exec(controller).unwrap();
}

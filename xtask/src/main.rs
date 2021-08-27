//! See <https://github.com/matklad/cargo-xtask/>.
//!
//! This binary defines various auxiliary build commands, which are not
//! expressible with just `cargo`.
//!
//! This binary is integrated into the `cargo` command line by using an alias in
//! `.cargo/config`.

use structopt::StructOpt;

mod ci;
mod test;

#[derive(StructOpt)]
enum Command {
    Test,
    Ci,
}

fn main() {
    match Command::from_args() {
        Command::Test => test::test(),
        Command::Ci => ci::ci(),
    }
}

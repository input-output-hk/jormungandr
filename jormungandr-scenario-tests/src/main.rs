#[macro_use]
extern crate jormungandr_scenario_tests;

use jormungandr_scenario_tests::{
    prepare_command, style,
    test::{comm::*, network::topology::scenarios::*},
    Context, Seed,
};
use std::{path::PathBuf, thread, time::Duration};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
struct CommandArgs {
    /// path or name of the jormungandr node to test
    #[structopt(long = "jormungandr", default_value = "jormungandr")]
    jormungandr: PathBuf,

    /// path or name of the jcli to test
    #[structopt(long = "jcli", default_value = "jcli")]
    jcli: PathBuf,

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

    /// to set if to reproduce an existing test
    #[structopt(long = "seed")]
    seed: Option<Seed>,
}

fn main() {
    let command_args = CommandArgs::from_args();

    let jormungandr = prepare_command(command_args.jormungandr);
    let jcli = prepare_command(command_args.jcli);
    let seed = command_args
        .seed
        .unwrap_or_else(|| Seed::generate(rand::rngs::OsRng::new().unwrap()));
    let testing_directory = command_args.testing_directory;
    let generate_documentation = command_args.generate_documentation;

    let mut context = Context::new(
        seed,
        jormungandr,
        jcli,
        testing_directory,
        generate_documentation,
    );

    introduction(&context);

    // tree(context.derive());
    relay(context.derive());
}

fn introduction<R: rand_core::RngCore>(context: &Context<R>) {
    println!(
        r###"
        ---_ ......._-_--.
       (|\ /      / /| \  \               _  ___  ____  __  __ _   _ _   _  ____    _    _   _ ____  ____
       /  /     .'  -=-'   `.            | |/ _ \|  _ \|  \/  | | | | \ | |/ ___|  / \  | \ | |  _ \|  _ \
      /  /    .'             )        _  | | | | | |_) | |\/| | | | |  \| | |  _  / _ \ |  \| | | | | |_) |
    _/  /   .'        _.)   /        | |_| | |_| |  _ <| |  | | |_| | |\  | |_| |/ ___ \| |\  | |_| |  _ <
   /   o  o       _.-' /  .'          \___/ \___/|_| \_\_|  |_|\___/|_| \_|\____/_/   \_\_| \_|____/|_| \_\
   \          _.-'    / .'#|
    \______.-'//    .'.' \#|         SCENARIO TEST SUITE
     \|  \ | //   .'.' _ |#|
      `   \|//  .'.'_._._|#|
       .  .// .'.' | _._ \#|
       \`-|\_/ /    \ _._ \#\
        `/'\__/      \ _._ \#\
       /^|            \ _-_ \#
      '  `             \ _-_ \
                        \_

 {}jormungandr: {}
 {}jcli:        {}
 {}seed:        {}

###############################################################################
    "###,
        *style::icons::jormungandr,
        style::binary.apply_to(context.jormungandr().to_string()),
        *style::icons::jcli,
        style::binary.apply_to(context.jcli().to_string()),
        *style::icons::seed,
        style::seed.apply_to(context.seed()),
    )
}

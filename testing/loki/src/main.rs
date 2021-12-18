use chain_core::property::Deserialize;
use chain_impl_mockchain::block::Block;
use jormungandr_lib::interfaces::NodeSecret;
use loki::{args::Args, error::Error, process::AdversaryNodeBuilder, rest::AdversaryRest};
use std::{fs::File, io::BufReader};
use structopt::StructOpt;

fn main() {
    let args = Args::from_args();

    if let Err(e) = launch(&args) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn launch(args: &Args) -> Result<(), Error> {
    let block0 = Block::deserialize(BufReader::new(File::open(&args.genesis_block)?))?;

    let mut rest = AdversaryRest::new(AdversaryNodeBuilder::new(block0).build());

    if let Some(secret_file) = args.secret.as_ref() {
        let secret: NodeSecret = serde_yaml::from_reader(BufReader::new(File::open(secret_file)?))?;

        if let Some(bft) = secret.bft {
            rest = rest.signing_key(bft.signing_key);
        }
    }

    if let Some(address) = args.listen_address {
        rest = rest.address(address);
    }

    rest.start();

    Ok(())
}

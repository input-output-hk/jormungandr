use cardano::util::hex;
use chain_core::property::Deserialize as _;
use chain_impl_mockchain::message::Message as MockMessage;
use jcli_app::utils;
use std::io::Read;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Message {
    /// file containing hex-encoded message. If not provided, it will be read from stdin.
    #[structopt(short, long)]
    input: Option<PathBuf>,
}

impl Message {
    pub fn exec(self) {
        let mut hex = String::new();
        utils::io::open_file_read(&self.input)
            .read_to_string(&mut hex)
            .unwrap();
        let bytes = hex::decode(&hex).unwrap();
        let message = MockMessage::deserialize(bytes.as_ref()).unwrap();
        println!("{:#?}", message);
    }
}

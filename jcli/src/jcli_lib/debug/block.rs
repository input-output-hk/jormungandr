use crate::jcli_lib::{debug::Error, utils::io};
use chain_core::{packer::Codec, property::Deserialize as _};
use chain_impl_mockchain::block::Block as BlockMock;
use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
};
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Block {
    /// file containing hex-encoded message. If not provided, it will be read from stdin.
    #[structopt(short, long)]
    input: Option<PathBuf>,
}

impl Block {
    pub fn exec(self) -> Result<(), Error> {
        let reader = io::open_file_read(&self.input).map_err(|source| Error::InputInvalid {
            source,
            path: self.input.unwrap_or_default(),
        })?;
        let mut hex_str = String::new();
        BufReader::new(reader).read_line(&mut hex_str)?;
        let bytes = hex::decode(hex_str.trim())?;
        let message = BlockMock::deserialize(&mut Codec::new(bytes.as_slice()))
            .map_err(Error::MessageMalformed)?;
        println!("{:#?}", message);
        Ok(())
    }
}

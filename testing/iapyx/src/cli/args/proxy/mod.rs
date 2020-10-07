use structopt::StructOpt;
use thiserror::Error;
use crate::backend::ProxyServerError;
use crate::backend::ProxyServerStub;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use jortestkit::file;

#[derive(Error, Debug)]
pub enum IapyxProxyCommandError {
    #[error("proxy error")]
    ProxyError(#[from] crate::backend::ProxyServerError),
}

#[derive(StructOpt, Debug)]
pub struct IapyxProxyCommand {

    #[structopt(short = "a", long = "address", default_value = "127.0.0.1:8000")]
    pub address: String,

    #[structopt(short = "v", long = "vit-address", default_value = "127.0.0.1:3030")]
    pub vit_address: String,
    
    #[structopt(short = "n", long = "node-address", default_value = "127.0.0.1:8080")]
    pub node_address: String,

    #[structopt(short = "b", long = "block0")]
    pub block0_path: PathBuf

}

impl IapyxProxyCommand {
    pub fn build(&self) -> Result<ProxyServerStub, IapyxProxyCommandError> {
        let proxy_address = self.address.clone();
        let vit_address = self.vit_address.clone();
        let node_address = self.node_address.clone();
        let block0_path = self.block0_path.clone();

        Ok(ProxyServerStub::new(proxy_address,vit_address,node_address,file::get_file_as_byte_vec(&block0_path))?)
    }
}



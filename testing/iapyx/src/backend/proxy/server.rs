use warp::{hyper::body::Bytes, Filter, Rejection, Reply};
use warp_reverse_proxy::reverse_proxy_filter;
use thiserror::Error;
use hyper::StatusCode;
use std::net::SocketAddr;
use url::{Url, ParseError};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Malformed proxy address: {0}")]
    MalformedProxyAddress(String),
    #[error("Malformed vit address: {0}")]
    MalformedVitStationAddress(String),
    #[error("Malformed node rest address: {0}")]
    MalformedNodeRestAddress(String),

}



pub struct ProxyServerStub{
    address: String,
    vit_address: String,
    node_rest_address: String,
    block0: Vec<u8>
}


impl ProxyServerStub{

    pub fn new(address: String, vit_address: String, node_rest_address: String, block0: Vec<u8>) -> Result<Self, Error> {
        Ok(Self { 
            address, 
            vit_address,
            node_rest_address,
            block0
        })
    }

    pub fn block0(&self) -> Vec<u8> {
        self.block0.clone()
    }

    pub fn address(&self) -> String {
        self.address.parse().unwrap()
    }

    pub fn vit_address(&self) -> String {
        self.vit_address.parse().unwrap()
    }

    pub fn node_rest_address(&self) -> String {
        self.node_rest_address.parse().unwrap()
    }

    pub fn base_address(&self) -> SocketAddr {
        self.address.parse().unwrap()
    }

    pub fn http_vit_address(&self) -> String {
        format!("http://{}/",self.vit_address)
    }
    
    pub fn http_node_address(&self) -> String {
        format!("http://{}/",self.node_rest_address)
    }

}
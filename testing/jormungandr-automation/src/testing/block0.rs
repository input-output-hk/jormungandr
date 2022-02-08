use chain_core::packer::Codec;
use chain_core::property::Deserialize;
use chain_core::property::DeserializeFromSlice;
use chain_impl_mockchain::block::Block;
use chain_impl_mockchain::certificate::VotePlan;
use jormungandr_lib::interfaces::Block0Configuration;
use jormungandr_lib::interfaces::Block0ConfigurationError;
use jormungandr_lib::interfaces::Initial;
use std::io::BufReader;
use std::path::Path;
use thiserror::Error;
use url::Url;

pub fn get_block<S: Into<String>>(block0: S) -> Result<Block0Configuration, GetBlock0Error> {
    let block0 = block0.into();
    let block = {
        if Path::new(&block0).exists() {
            let reader = std::fs::OpenOptions::new()
                .create(false)
                .write(false)
                .read(true)
                .append(false)
                .open(&block0)?;
            let reader = BufReader::new(reader);
            Block::deserialize(&mut Codec::new(reader))?
        } else if Url::parse(&block0).is_ok() {
            let response = reqwest::blocking::get(&block0)?;
            let block0_bytes = response.bytes()?.to_vec();
            Block::deserialize_from_slice(&mut Codec::new(block0_bytes.as_slice()))?
        } else {
            panic!(" block0 should be either path to filesystem or url ");
        }
    };
    Block0Configuration::from_block(&block).map_err(Into::into)
}

pub trait Block0ConfigurationExtension {
    fn vote_plans(&self) -> Vec<VotePlan>;
}

impl Block0ConfigurationExtension for Block0Configuration {
    fn vote_plans(&self) -> Vec<VotePlan> {
        let mut vote_plans = vec![];
        for initial in self.initial.iter().cloned() {
            if let Initial::Cert(cert) = initial {
                if let chain_impl_mockchain::certificate::Certificate::VotePlan(vote_plan) =
                    cert.strip_auth().0.clone()
                {
                    vote_plans.push(vote_plan.clone());
                }
            }
        }
        vote_plans
    }
}

#[derive(Error, Debug)]
pub enum GetBlock0Error {
    #[error("reqwest error")]
    IapyxStatsCommandError(#[from] reqwest::Error),
    #[error("block0 parse error")]
    Block0ParseError(#[from] Block0ConfigurationError),
    #[error("io error")]
    IoError(#[from] std::io::Error),
    #[error("read error")]
    ReadError(#[from] chain_core::property::ReadError),
    #[error("bech32 error")]
    Bech32Error(#[from] bech32::Error),
}

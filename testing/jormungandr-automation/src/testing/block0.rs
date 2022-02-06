use chain_core::{
    mempack::{ReadBuf, Readable},
    property::{Block as _, Deserialize, Serialize},
};
use chain_impl_mockchain::{block::Block, certificate::VotePlan, ledger::Ledger};
use jormungandr_lib::interfaces::{Block0Configuration, Block0ConfigurationError, Initial};
use std::{io::BufReader, path::Path};
use thiserror::Error;
use url::Url;

pub fn get_block<S: Into<String>>(block0: S) -> Result<Block0Configuration, Block0Error> {
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
            Block::deserialize(reader)?
        } else if Url::parse(&block0).is_ok() {
            let response = reqwest::blocking::get(&block0)?;
            let block0_bytes = response.bytes()?.to_vec();
            Block::read(&mut ReadBuf::from(&block0_bytes))?
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

pub fn read_genesis_yaml<P: AsRef<Path>>(genesis: P) -> Result<Block0Configuration, Block0Error> {
    let contents = std::fs::read_to_string(&genesis)?;
    serde_yaml::from_str(&contents).map_err(Into::into)
}

pub fn read_initials<P: AsRef<Path>>(initials: P) -> Result<Vec<Initial>, Block0Error> {
    let contents = std::fs::read_to_string(&initials)?;
    let value: serde_json::Value = serde_json::from_str(&contents)?;
    let initial = serde_json::to_string(&value["initial"])?;
    serde_json::from_str(&initial).map_err(Into::into)
}

pub fn write_genesis_yaml<P: AsRef<Path>>(
    genesis: Block0Configuration,
    path: P,
) -> Result<(), Block0Error> {
    use std::io::Write;
    let content = serde_yaml::to_string(&genesis)?;
    let mut file = std::fs::File::create(&path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

pub fn encode_block0<P: AsRef<Path>, Q: AsRef<Path>>(
    genesis: P,
    block0: Q,
) -> Result<(), Block0Error> {
    let input: std::fs::File = std::fs::OpenOptions::new()
        .create(false)
        .write(false)
        .read(true)
        .append(false)
        .truncate(false)
        .open(&genesis)?;

    let output: std::fs::File = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .read(false)
        .append(false)
        .truncate(true)
        .open(&block0)?;

    let genesis: Block0Configuration = serde_yaml::from_reader(input)?;
    let block = genesis.to_block();
    Ledger::new(block.id(), block.fragments())?;
    block.serialize(&output).map_err(Into::into)
}

pub fn decode_block0<Q: AsRef<Path>>(block0: Vec<u8>, genesis_yaml: Q) -> Result<(), Block0Error> {
    let writer: std::fs::File = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .read(false)
        .append(false)
        .truncate(true)
        .open(&genesis_yaml)?;

    let yaml = Block0Configuration::from_block(&Block::deserialize(&*block0)?)?;
    Ok(serde_yaml::to_writer(writer, &yaml)?)
}

#[derive(Error, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Block0Error {
    #[error(transparent)]
    IapyxStatsCommandError(#[from] reqwest::Error),
    #[error(transparent)]
    Block0ParseError(#[from] Block0ConfigurationError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ReadError(#[from] chain_core::mempack::ReadError),
    #[error(transparent)]
    Bech32Error(#[from] bech32::Error),
    #[error(transparent)]
    SerdeYaml(#[from] serde_yaml::Error),
    #[error(transparent)]
    Ledger(#[from] chain_impl_mockchain::ledger::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
}

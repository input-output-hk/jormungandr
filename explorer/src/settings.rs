use chain_impl_mockchain::block::HeaderId;
use jormungandr_lib::interfaces::{Cors, Tls};
use serde::{de, Deserialize};
use std::{fs::File, net::SocketAddr, path::PathBuf};
use structopt::StructOpt;
use thiserror::Error;
use tonic::transport::Uri;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Format(#[from] serde_yaml::Error),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error("Invalid host")]
    InvalidHost,
}

#[derive(Debug)]
pub struct Settings {
    pub node: Uri,
    pub block0_hash: HeaderId,
    pub binding_address: SocketAddr,
    pub address_bech32_prefix: String,
    pub tls: Option<Tls>,
    pub cors: Option<Cors>,
}

impl Settings {
    pub fn load() -> Result<Settings, Error> {
        let cmd = CommandLine::from_args();
        let file: Config = cmd
            .config
            .map(|file_path| -> Result<Config, Error> {
                serde_yaml::from_reader(File::open(file_path)?).map_err(Error::from)
            })
            .transpose()?
            .unwrap_or_default();

        let node = cmd
            .node
            .or(file.host)
            .unwrap_or("127.0.0.1:8299".parse().unwrap());

        let block0_hash = cmd.block0_hash.parse().unwrap();

        let binding_address = cmd
            .binding_address
            .or(file.binding_address)
            .unwrap_or("0.0.0.0:3030".parse().unwrap());

        let address_bech32_prefix = cmd
            .address_bech32_prefix
            .or(file.address_bech32_prefix.clone())
            .unwrap_or("addr".to_string());

        let tls = file.tls;
        let cors = file.cors;

        Ok(Settings {
            node,
            block0_hash,
            binding_address,
            address_bech32_prefix,
            tls,
            cors,
        })
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "config")]
struct CommandLine {
    #[structopt(long)]
    pub node: Option<Uri>,
    pub block0_hash: String,
    #[structopt(long)]
    pub binding_address: Option<SocketAddr>,
    #[structopt(long)]
    pub address_bech32_prefix: Option<String>,
    pub config: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub storage: Option<PathBuf>,
    pub tls: Option<Tls>,
    pub cors: Option<Cors>,
    #[serde(default, deserialize_with = "deserialize_uri_string")]
    pub host: Option<Uri>,
    pub binding_address: Option<SocketAddr>,
    pub address_bech32_prefix: Option<String>,
}

fn deserialize_uri_string<'de, D>(deserializer: D) -> Result<Option<Uri>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s: &str = de::Deserialize::deserialize(deserializer)?;
    Ok(Some(s.parse().unwrap()))
}

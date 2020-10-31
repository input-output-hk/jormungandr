use jormungandr_lib::interfaces::{Cors, Tls};
use serde::Deserialize;
use std::{fs::File, net::SocketAddr, path::PathBuf};
use structopt::StructOpt;
use thiserror::Error;

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
    pub host: url::Host,
    pub port: u16,
    pub block0_hash: String,
    pub binding_address: SocketAddr,
    pub address_bech32_prefix: String,
    pub tls: Option<Tls>,
    pub cors: Option<Cors>,
}

impl Settings {
    pub fn load() -> Result<Settings, Error> {
        let cmd = CommandLine::from_args();
        let file: Option<Config> = cmd
            .config
            .map(|file_path| -> Result<Config, Error> {
                serde_yaml::from_reader(File::open(file_path)?).map_err(Error::from)
            })
            .transpose()?;

        let file_host = file
            .as_ref()
            .and_then(|f| f.host.as_ref())
            .map(|host| url::Host::parse(&host))
            .transpose()
            .map_err(|_| Error::InvalidHost)?;

        let host = cmd
            .host
            .or(file_host)
            .unwrap_or(url::Host::parse("localhost").unwrap());

        let port = cmd
            .port
            .or(file.as_ref().and_then(|f| f.port))
            .unwrap_or(8443);

        let block0_hash = cmd.block0_hash;

        let binding_address = cmd
            .binding_address
            .or(file.as_ref().and_then(|f| f.binding_address))
            .unwrap_or("0.0.0.0:3030".parse().unwrap());

        let address_bech32_prefix = cmd
            .address_bech32_prefix
            .or(file.as_ref().and_then(|f| f.address_bech32_prefix.clone()))
            .unwrap_or("addr".to_string());

        let tls = file.as_ref().and_then(|settings| settings.tls.clone());
        let cors = file.as_ref().and_then(|settings| settings.cors.clone());

        Ok(Settings {
            host,
            port,
            block0_hash,
            binding_address,
            address_bech32_prefix,
            tls,
            cors,
        })
    }

    pub fn rest_url(&self) -> url::Url {
        // TODO: this can't fail I think, there must be some way of removing this expect
        url::Url::parse(&format!("http://{}:{}/api/", self.host, self.port))
            .expect("couldn't form rest url")
    }

    pub fn notifier_url(&self) -> url::Url {
        url::Url::parse(&format!("ws://{}:{}/api/", self.host, self.port))
            .expect("couldn't form base url")
            .join("v1/notifier")
            .unwrap()
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "config")]
struct CommandLine {
    #[structopt(long, parse(try_from_str = url::Host::parse))]
    pub host: Option<url::Host>,
    #[structopt(long)]
    pub port: Option<u16>,
    pub block0_hash: String,
    #[structopt(long)]
    pub binding_address: Option<SocketAddr>,
    #[structopt(long)]
    pub address_bech32_prefix: Option<String>,
    pub config: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub storage: Option<PathBuf>,
    pub tls: Option<Tls>,
    pub cors: Option<Cors>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub binding_address: Option<SocketAddr>,
    pub address_bech32_prefix: Option<String>,
}

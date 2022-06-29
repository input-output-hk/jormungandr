use crate::wallet::discrimination::DiscriminationExtension;
use chain_addr::{Address, AddressReadable, Discrimination, Kind};
use chain_crypto::{bech32::Bech32, Ed25519, PublicKey};
use chain_impl_mockchain::account::Identifier;
use jormungandr_automation::jormungandr::{JormungandrRest, RestSettings};
use jormungandr_lib::crypto::hash::Hash;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::{collections::HashMap, io::Write, path::PathBuf};
use thiserror::Error;

pub type Alias = String;
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub connection: Connection,
    pub wallets: Wallets,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub address: String,
    pub https: bool,
    pub debug: bool,
}

#[allow(clippy::from_over_into)]
impl Into<JormungandrRest> for Connection {
    fn into(self) -> JormungandrRest {
        JormungandrRest::new_with_custom_settings(self.address.clone(), self.into())
    }
}

#[allow(clippy::from_over_into)]
impl Into<RestSettings> for Connection {
    fn into(self) -> RestSettings {
        RestSettings {
            enable_debug: self.debug,
            use_https: self.https,
            certificate: None,
            cors: None,
        }
    }
}

impl Default for Connection {
    fn default() -> Self {
        Connection {
            address: "http://localhost".to_string(),
            https: false,
            debug: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct SecretKey {
    pub(crate) secret: String,
}

impl std::str::FromStr for SecretKey {
    type Err = Error;

    fn from_str(key: &str) -> std::result::Result<Self, Error> {
        Ok(Self {
            secret: key.to_owned(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Wallets {
    pub(crate) default: Option<Alias>,
    pub wallets: HashMap<Alias, WalletState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletState {
    pub pending_tx: Vec<Hash>,
    pub public_key: String,
    pub spending_counters: Vec<u32>,
    //path to secret key in format of SecretKey struct
    pub secret_file: PathBuf,
    pub testing: bool,
    pub value: u64,
}

impl WalletState {
    pub fn address(&self) -> Result<Address, Error> {
        let kind = Kind::Account(self.pk()?);
        Ok(Address(self.discrimination(), kind))
    }

    pub fn discrimination(&self) -> Discrimination {
        if self.testing {
            Discrimination::Test
        } else {
            Discrimination::Production
        }
    }

    pub fn address_readable(&self) -> Result<AddressReadable, Error> {
        self.address()
            .map(|a| AddressReadable::from_address(&self.discrimination().into_prefix(), &a))
    }

    pub fn pk(&self) -> Result<PublicKey<Ed25519>, Error> {
        Bech32::try_from_bech32_str(&self.pk_bech32()).map_err(Into::into)
    }

    pub fn pk_bech32(&self) -> String {
        self.public_key.clone()
    }

    pub fn account_id(&self) -> Result<Identifier, Error> {
        Ok(Identifier::from(self.pk()?))
    }

    pub fn id(&self) -> Result<String, Error> {
        Ok(hex::encode(self.account_id()?.as_ref()))
    }
}

pub struct ConfigManager {
    app_name: String,
}

impl ConfigManager {
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
        }
    }

    pub fn save_config(&self, config: &Config) -> Result<(), Error> {
        let content = serde_yaml::to_string(config)?;
        std::fs::write(self.config_file()?, content).map_err(Into::into)
    }

    pub fn alias_secret_file(&self, alias: &Alias) -> Result<PathBuf, Error> {
        let filename = format!("{}.secret", alias);
        Ok(self.app_dir()?.join(filename))
    }

    pub fn read_config(&self) -> Result<Config, Error> {
        let app_dir = self.app_dir()?;
        let config_file = self.config_file()?;

        if !config_file.exists() {
            std::fs::create_dir_all(&app_dir)
                .map_err(|_| Error::CannotCreateConfigFileFolder(config_file.to_path_buf()))?;
            let mut file = std::fs::File::create(&config_file)
                .map_err(|_| Error::CannotCreateConfigFile(config_file.to_path_buf()))?;
            let content = serde_yaml::to_string(&Config::default())?;
            file.write_all(content.as_bytes())?;
        }
        serde_yaml::from_str(
            &std::fs::read_to_string(&config_file)
                .map_err(|_| Error::CannotReadConfigFile(config_file.to_path_buf()))?,
        )
        .map_err(Error::CannotDeserializeConfigFile)
    }

    pub fn app_dir(&self) -> Result<PathBuf, Error> {
        let home_folder = dirs::home_dir().ok_or(Error::CannotRetrieveHomeDir)?;
        Ok(home_folder.join(".".to_owned() + &self.app_name))
    }

    pub fn config_file(&self) -> Result<PathBuf, Error> {
        Ok(self.app_dir()?.join("config.yaml"))
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot create config file in: {0}")]
    CannotCreateConfigFile(PathBuf),
    #[error("cannot read config file from: {0}")]
    CannotReadConfigFile(PathBuf),
    #[error("cannot deserialize config file")]
    CannotDeserializeConfigFile(#[from] serde_yaml::Error),
    #[error("cannot retrieve user home dir")]
    CannotRetrieveHomeDir,
    #[error("cannot create config folder: {0}")]
    CannotCreateConfigFileFolder(PathBuf),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Bech32(#[from] chain_crypto::bech32::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode;
    use cocoon::Cocoon;
    use std::fs::File;

    #[test]
    pub fn secure_config() {
        let secret_config = SecretKey {
            secret: "ed25519_sk1lv8m52aakjm2z0yqtltqxp6f9kemtcmt9ej7gn86eru7gpt8tn0q08j8lt"
                .to_string(),
        };

        let encoded: Vec<u8> = bincode::serialize(&secret_config).unwrap();

        let cocoon = Cocoon::new(b"password");
        let filename = "secret.key";
        {
            let mut file = File::create(filename).unwrap();
            cocoon.dump(encoded, &mut file).unwrap();
        }

        let mut file = File::open(filename).unwrap();
        let encoded = cocoon.parse(&mut file).unwrap();

        let decoded: SecretKey = bincode::deserialize(&encoded[..]).unwrap();
        assert_eq!(secret_config, decoded);
    }
}

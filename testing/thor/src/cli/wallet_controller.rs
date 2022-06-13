use super::{
    config::{Alias, Connection, WalletState},
    Config, ConfigManager, Error,
};
use chain_crypto::Ed25519Extended;
use chain_impl_mockchain::fragment::FragmentId;
use cocoon::Cocoon;
use jcli_lib::key::gen_pub_key;
use jormungandr_lib::interfaces::FragmentStatus;
use std::{collections::HashMap, fs::File};

pub struct WalletController {
    config: Config,
    config_manager: ConfigManager,
}

impl WalletController {
    pub fn new(app_name: &str) -> Result<Self, Error> {
        Self::new_from_manager(ConfigManager::new(app_name))
    }

    pub fn new_from_manager(config_manager: ConfigManager) -> Result<Self, Error> {
        Ok(Self {
            config: config_manager.read_config()?,
            config_manager,
        })
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, Alias, WalletState> {
        self.config.wallets.wallets.iter()
    }

    pub fn connection(&self) -> Connection {
        self.config.connection.clone()
    }

    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    pub fn set_default_alias(&mut self, alias: Alias) -> Result<(), Error> {
        if self.alias_exists(&alias) {
            self.config.wallets.default = Some(alias);
            Ok(())
        } else {
            Err(Error::UknownAlias(alias))
        }
    }

    pub fn is_default_alias(&mut self, alias: &Alias) -> bool {
        self.config.wallets.default.as_ref() == Some(alias)
    }

    pub fn default_alias(&self) -> Option<&Alias> {
        self.config.wallets.default.as_ref()
    }

    pub fn alias_exists(&self, alias: &Alias) -> bool {
        self.config.wallets.wallets.contains_key(alias)
    }

    pub fn remove_wallet(&mut self, alias: Alias) -> Result<(), Error> {
        if self.alias_exists(&alias) {
            self.config.wallets.wallets.retain(|x, _| x != &alias);
            std::fs::remove_file(self.config_manager.alias_secret_file(&alias)?).map_err(Into::into)
        } else {
            Err(Error::UknownAlias(alias))
        }
    }

    pub fn wallet(&self) -> Result<WalletState, Error> {
        match &self.config.wallets.default {
            None => Err(Error::NoDefaultAliasDefined),
            Some(alias) => {
                if !self.alias_exists(alias) {
                    return Err(Error::UknownAlias(alias.to_string()));
                }
                Ok(self.config.wallets.wallets.get(alias).unwrap().clone())
            }
        }
    }

    pub fn wallet_mut(&mut self) -> Result<&mut WalletState, Error> {
        match &self.config.wallets.default {
            None => Err(Error::NoDefaultAliasDefined),
            Some(alias) => {
                if !self.alias_exists(alias) {
                    return Err(Error::UknownAlias(alias.to_string()));
                }
                Ok(self.config.wallets.wallets.get_mut(alias).unwrap())
            }
        }
    }

    pub fn confirm_txs(
        &mut self,
        statuses: HashMap<FragmentId, FragmentStatus>,
    ) -> Result<(), Error> {
        let wallet = self.wallet_mut()?;

        wallet.pending_tx.retain(|x| {
            if let Some(status) = statuses.get(&x.into_hash()) {
                !status.is_pending()
            } else {
                false
            }
        });
        Ok(())
    }

    pub fn clear_txs(&mut self) -> Result<(), Error> {
        self.wallet_mut()?.pending_tx.clear();
        Ok(())
    }

    pub fn add_wallet(
        &mut self,
        alias: Alias,
        testing: bool,
        data: Vec<bech32::u5>,
        password: &str,
    ) -> Result<(), Error> {
        let cocoon = Cocoon::new(password.as_bytes());
        let secret_file = self.config_manager.alias_secret_file(&alias)?;
        let mut file = File::create(&secret_file)?;

        let data_u8 = data.iter().map(|x| x.to_u8()).collect();
        cocoon.dump(data_u8, &mut file)?;

        let public_key = gen_pub_key::<Ed25519Extended>(&data)?;
        self.config.wallets.wallets.insert(
            alias,
            WalletState {
                public_key,
                secret_file,
                pending_tx: Vec::new(),
                testing,
                spending_counters: vec![
                    0, 536870912, 1073741824, 1610612736, 2147483648, 2684354560, 3221225472,
                    3758096384,
                ],
                value: 0,
            },
        );

        Ok(())
    }

    pub fn save_config(&self) -> Result<(), Error> {
        self.config_manager
            .save_config(&self.config)
            .map_err(Into::into)
    }
}

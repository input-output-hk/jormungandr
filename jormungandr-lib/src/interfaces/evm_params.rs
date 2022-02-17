use std::convert::{TryFrom, TryInto};

use chain_impl_mockchain::config;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum EvmConfig {
    Frontier,
    Istanbul,
    Berlin,
    London,
}

impl From<config::EvmConfig> for EvmConfig {
    fn from(val: config::EvmConfig) -> Self {
        match val {
            config::EvmConfig::Frontier => Self::Frontier,
            config::EvmConfig::Istanbul => Self::Istanbul,
            config::EvmConfig::Berlin => Self::Berlin,
            config::EvmConfig::London => Self::London,
        }
    }
}

impl From<EvmConfig> for config::EvmConfig {
    fn from(val: EvmConfig) -> Self {
        match val {
            EvmConfig::Frontier => Self::Frontier,
            EvmConfig::Istanbul => Self::Istanbul,
            EvmConfig::Berlin => Self::Berlin,
            EvmConfig::London => Self::London,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct EvmEnvSettings {
    gas_price: u64,
    block_gas_limit: u64,
}

#[derive(Debug, Error)]
pub enum TryFromEvmEnvSettingsError {
    #[error("Incompatible Config param, expected EvmEnvSettings")]
    Incompatible,
}

impl TryFrom<config::EvmEnvSettings> for EvmEnvSettings {
    type Error = TryFromEvmEnvSettingsError;
    fn try_from(val: config::EvmEnvSettings) -> Result<Self, Self::Error> {
        Ok(Self {
            gas_price: val
                .gas_price
                .try_into()
                .map_err(|_| TryFromEvmEnvSettingsError::Incompatible)?,
            block_gas_limit: val
                .block_gas_limit
                .try_into()
                .map_err(|_| TryFromEvmEnvSettingsError::Incompatible)?,
        })
    }
}

impl From<EvmEnvSettings> for config::EvmEnvSettings {
    fn from(val: EvmEnvSettings) -> Self {
        Self {
            gas_price: val.gas_price.into(),
            block_gas_limit: val.block_gas_limit.into(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::Arbitrary;

    impl Arbitrary for EvmConfig {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            config::EvmConfig::arbitrary(g).into()
        }
    }

    impl Arbitrary for EvmEnvSettings {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            Self {
                gas_price: Arbitrary::arbitrary(g),
                block_gas_limit: Arbitrary::arbitrary(g),
            }
        }
    }

    quickcheck! {
        fn evm_config_params_bincode_serde_test(evm_params: EvmConfig) -> bool {
            let decoded_evm_params: EvmConfig = bincode::deserialize(bincode::serialize(&evm_params).unwrap().as_slice()).unwrap();
            decoded_evm_params == evm_params
        }

        fn evm_config_params_yaml_serde_test(evm_params: EvmConfig) -> bool {
            let decoded_evm_params: EvmConfig = serde_yaml::from_str(&serde_yaml::to_string(&evm_params).unwrap()).unwrap();
            decoded_evm_params == evm_params
        }

        fn evm_env_settings_params_bincode_serde_test(evm_params: EvmEnvSettings) -> bool {
            let decoded_evm_params: EvmEnvSettings = bincode::deserialize(bincode::serialize(&evm_params).unwrap().as_slice()).unwrap();
            decoded_evm_params == evm_params
        }

        fn evm_env_settings_params_yaml_serde_test(evm_params: EvmEnvSettings) -> bool {
            let decoded_evm_params: EvmEnvSettings = serde_yaml::from_str(&serde_yaml::to_string(&evm_params).unwrap()).unwrap();
            decoded_evm_params == evm_params
        }
    }
}

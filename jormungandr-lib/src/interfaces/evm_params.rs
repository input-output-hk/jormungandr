use chain_impl_mockchain::{config, evm};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum EvmConfig {
    Frontier,
    Istanbul,
    Berlin,
    London,
}

impl From<evm::Config> for EvmConfig {
    fn from(val: evm::Config) -> Self {
        match val {
            evm::Config::Frontier => Self::Frontier,
            evm::Config::Istanbul => Self::Istanbul,
            evm::Config::Berlin => Self::Berlin,
            evm::Config::London => Self::London,
        }
    }
}

impl From<EvmConfig> for evm::Config {
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

impl From<config::EvmEnvSettings> for EvmEnvSettings {
    fn from(val: config::EvmEnvSettings) -> Self {
        Self {
            gas_price: val.gas_price,
            block_gas_limit: val.block_gas_limit,
        }
    }
}

impl From<EvmEnvSettings> for config::EvmEnvSettings {
    fn from(val: EvmEnvSettings) -> Self {
        Self {
            gas_price: val.gas_price,
            block_gas_limit: val.block_gas_limit,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::Arbitrary;

    impl Arbitrary for EvmConfig {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            evm::Config::arbitrary(g).into()
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

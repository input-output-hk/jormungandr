use chain_impl_mockchain::evm;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum EvmConfig {
    Istanbul,
    Berlin,
}

impl From<evm::Config> for EvmConfig {
    fn from(val: evm::Config) -> Self {
        match val {
            evm::Config::Istanbul => Self::Istanbul,
            evm::Config::Berlin => Self::Berlin,
            _ => unimplemented!(),
        }
    }
}

impl From<EvmConfig> for evm::Config {
    fn from(val: EvmConfig) -> Self {
        match val {
            EvmConfig::Istanbul => Self::Istanbul,
            EvmConfig::Berlin => Self::Berlin,
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

    quickcheck! {
        fn evm_config_params_bincode_serde_test(evm_params: EvmConfig) -> bool {
            let decoded_evm_params: EvmConfig = bincode::deserialize(bincode::serialize(&evm_params).unwrap().as_slice()).unwrap();
            decoded_evm_params == evm_params
        }

        fn evm_config_params_yaml_serde_test(evm_params: EvmConfig) -> bool {
            let decoded_evm_params: EvmConfig = serde_yaml::from_str(&serde_yaml::to_string(&evm_params).unwrap()).unwrap();
            decoded_evm_params == evm_params
        }
    }
}

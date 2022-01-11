use chain_impl_mockchain::config;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Default)]
pub struct EvmConfigParams {
    #[serde(skip)]
    evm_config: config::EvmConfigParams,
}

impl From<config::EvmConfigParams> for EvmConfigParams {
    fn from(val: config::EvmConfigParams) -> Self {
        Self { evm_config: val }
    }
}

impl From<EvmConfigParams> for config::EvmConfigParams {
    fn from(val: EvmConfigParams) -> Self {
        val.evm_config
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::Arbitrary;

    impl Arbitrary for EvmConfigParams {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            Self {
                evm_config: Arbitrary::arbitrary(g),
            }
        }
    }

    quickcheck! {
        fn evm_config_params_bincode_serde_test(evm_params: EvmConfigParams) -> bool {
            let decoded_evm_params: EvmConfigParams = bincode::deserialize(bincode::serialize(&evm_params).unwrap().as_slice()).unwrap();
            decoded_evm_params == evm_params
        }

        fn evm_config_params_yaml_serde_test(evm_params: EvmConfigParams) -> bool {
            let decoded_evm_params: EvmConfigParams = serde_yaml::from_str(&serde_yaml::to_string(&evm_params).unwrap()).unwrap();
            decoded_evm_params == evm_params
        }
    }
}

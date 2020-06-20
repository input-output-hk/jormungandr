use super::Version;
use crate::common::configuration::JormungandrParams;
use jormungandr_lib::interfaces::NodeConfig as NewestNodeConfig;
use jormungandr_testing_utils::legacy::{NodeConfig, P2p, Rest, TrustedPeer};
use rand::RngCore;
use rand_core::OsRng;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LegacyConfigConverterError {
    #[error("unsupported version")]
    UnsupportedVersion(Version),
}

pub const fn version_0_8_19() -> Version {
    Version::new(0, 8, 19)
}

/// Used to build configuration for legacy nodes.
/// It uses yaml_rust instead of serde yaml serializer
/// beacuse config model is always up to date with newest config schema
/// while legacy node requires na old one
pub struct LegacyConfigConverter {
    version: Version,
}

impl LegacyConfigConverter {
    pub fn new(version: Version) -> Self {
        Self { version }
    }

    pub fn convert(
        &self,
        params: JormungandrParams<NewestNodeConfig>,
    ) -> Result<JormungandrParams<NodeConfig>, LegacyConfigConverterError> {
        if self.version > version_0_8_19() {
            return Err(LegacyConfigConverterError::UnsupportedVersion(
                self.version.clone(),
            ));
        }

        let node_config_converter = LegacyNodeConfigConverter::new(self.version.clone());
        let node_config = node_config_converter.convert(params.node_config())?;
        Ok(self.build_configuration_before_0_8_19(params, node_config))
    }

    fn build_configuration_before_0_8_19(
        &self,
        params: JormungandrParams<NewestNodeConfig>,
        backward_compatible_config: NodeConfig,
    ) -> JormungandrParams<NodeConfig> {
        JormungandrParams::new(
            backward_compatible_config,
            params.node_config_path(),
            params.genesis_block_path(),
            params.genesis_block_hash(),
            params.secret_model_paths(),
            params.block0_configuration().clone(),
            params.rewards_history(),
            params.log_file_path(),
        )
    }
}

pub struct LegacyNodeConfigConverter {
    version: Version,
}

impl LegacyNodeConfigConverter {
    pub fn new(version: Version) -> Self {
        Self { version }
    }

    pub fn convert(
        &self,
        source: &NewestNodeConfig,
    ) -> Result<NodeConfig, LegacyConfigConverterError> {
        if self.version > version_0_8_19() {
            return Err(LegacyConfigConverterError::UnsupportedVersion(
                self.version.clone(),
            ));
        }
        Ok(self.build_node_config_before_0_8_19(source))
    }

    fn generate_legacy_poldercast_id(rng: &mut OsRng) -> String {
        let mut bytes: [u8; 24] = [0; 24];
        rng.fill_bytes(&mut bytes);
        hex::encode(&bytes)
    }

    fn build_node_config_before_0_8_19(&self, source: &NewestNodeConfig) -> NodeConfig {
        let mut rng = OsRng;
        let trusted_peers: Vec<TrustedPeer> = source
            .p2p
            .trusted_peers
            .iter()
            .map(|peer| {
                let id = {
                    if let Some(id) = peer.id {
                        id.to_string()
                    } else {
                        Self::generate_legacy_poldercast_id(&mut rng)
                    }
                };

                TrustedPeer {
                    id: Some(id),
                    address: peer.address.clone(),
                }
            })
            .collect();

        NodeConfig {
            storage: source.storage.clone(),
            log: source.log.clone(),
            rest: Rest {
                listen: source.rest.listen,
            },
            p2p: P2p {
                trusted_peers,
                public_address: source.p2p.public_address.clone(),
                listen_address: None,
                max_inbound_connections: None,
                max_connections: None,
                topics_of_interest: source.p2p.topics_of_interest.clone(),
                allow_private_addresses: source.p2p.allow_private_addresses,
                policy: source.p2p.policy.clone(),
                layers: None,
                public_id: None,
            },
            mempool: source.mempool.clone(),
            explorer: source.explorer.clone(),
            bootstrap_from_trusted_peers: source.bootstrap_from_trusted_peers,
            skip_bootstrap: source.skip_bootstrap,
        }
    }
}

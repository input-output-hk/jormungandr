use super::BackwardCompatibleConfig;
use super::Version;
use crate::common::{
    configuration::JormungandrConfig, file_utils, jormungandr::starter::StartupError,
};
use hex;
use jormungandr_lib::interfaces::NodeConfig as NewestNodeConfig;
use jormungandr_testing_utils::legacy::{NodeConfig, P2p, Rest, TrustedPeer};
use rand::RngCore;
use rand_core::OsRng;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LegacyConfigConverterError {
    #[error("unsupported version")]
    UnsupportedVersion(Version),
    #[error("serialization error")]
    SpawnError(#[from] StartupError),
}

fn version_08_19() -> Version {
    Version::new(8, 19, 0)
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
        config: JormungandrConfig,
    ) -> Result<BackwardCompatibleConfig, LegacyConfigConverterError> {
        if self.version > version_08_19() {
            return Err(LegacyConfigConverterError::UnsupportedVersion(
                self.version.clone(),
            ));
        }

        let node_config_converter = LegacyNodeConfigConverter::new(self.version.clone());
        let node_config = node_config_converter.convert(config.node_config())?;
        Ok(self.build_configuration_before_08_19(config, node_config))
    }

    pub fn build_configuration_before_08_19(
        &self,
        config: JormungandrConfig,
        backward_compatible_config: NodeConfig,
    ) -> BackwardCompatibleConfig {
        let _source = config.node_config();

        let content = serde_yaml::to_string(&backward_compatible_config)
            .expect("cannot serializer node config before 08.19 version");
        let node_config_path = file_utils::create_file_in_temp("node_config.xml", &content);

        BackwardCompatibleConfig::new(
            config.genesis_block_path().clone(),
            config.genesis_block_hash().clone(),
            node_config_path,
            config.secret_model_paths().clone(),
            config.block0_configuration().clone(),
            config.secret_models().clone(),
            config.rewards_history(),
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
        source: NewestNodeConfig,
    ) -> Result<NodeConfig, LegacyConfigConverterError> {
        if self.version > version_08_19() {
            return Err(LegacyConfigConverterError::UnsupportedVersion(
                self.version.clone(),
            ));
        }
        Ok(self.build_node_config_before_08_19(source))
    }

    fn generate_legacy_poldercast_id(rng: &mut OsRng) -> String {
        let mut bytes: [u8; 24] = [0; 24];
        rng.fill_bytes(&mut bytes);
        hex::encode(&bytes).to_string()
    }

    fn build_node_config_before_08_19(&self, source: NewestNodeConfig) -> NodeConfig {
        let mut rng = OsRng;
        let trusted_peers: Vec<TrustedPeer> = source
            .p2p
            .trusted_peers
            .iter()
            .map(|peer| TrustedPeer {
                id: Some(Self::generate_legacy_poldercast_id(&mut rng)),
                address: peer.address.clone(),
            })
            .collect();

        NodeConfig {
            storage: source.storage.clone(),
            log: source.log.clone(),
            rest: Rest {
                listen: source.rest.listen.clone(),
            },
            p2p: P2p {
                trusted_peers: trusted_peers,
                public_address: source.p2p.public_address.clone(),
                listen_address: None,
                max_inbound_connections: None,
                max_connections: None,
                topics_of_interest: source.p2p.topics_of_interest.clone(),
                allow_private_addresses: false,
                policy: source.p2p.policy.clone(),
                layers: None,
            },
            mempool: source.mempool.clone(),
            explorer: source.explorer.clone(),
            bootstrap_from_trusted_peers: source.bootstrap_from_trusted_peers,
            skip_bootstrap: source.skip_bootstrap,
        }
    }
}

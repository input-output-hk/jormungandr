use super::{P2p, TrustedPeer};
use crate::jormungandr::{
    legacy::{config::NodeConfig, version_0_13_0},
    JormungandrParams, Version,
};
use jormungandr_lib::interfaces::{NodeConfig as NewestNodeConfig, NodeId, Rest};
use rand::RngCore;
use rand_core::OsRng;
use thiserror::Error;
#[derive(Error, Debug)]
pub enum LegacyConfigConverterError {
    #[error("unsupported version")]
    UnsupportedVersion(Version),
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
        let node_config_converter = LegacyNodeConfigConverter::new(self.version.clone());
        let node_config = node_config_converter.convert(params.node_config())?;
        Ok(self.build_configuration(params, node_config))
    }

    fn build_configuration(
        &self,
        params: JormungandrParams<NewestNodeConfig>,
        backward_compatible_config: NodeConfig,
    ) -> JormungandrParams<NodeConfig> {
        JormungandrParams::new(
            backward_compatible_config,
            params.node_config_path(),
            params.genesis_block_path(),
            params.genesis_block_hash(),
            params.secret_model_path(),
            params.block0_configuration().clone(),
            params.rewards_history(),
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

    ///0.8.19 is a breaking point where in trusted peer id was obsoleted
    pub fn convert(
        &self,
        source: &NewestNodeConfig,
    ) -> Result<NodeConfig, LegacyConfigConverterError> {
        if self.version >= version_0_13_0() {
            return Ok(self.build_node_config_after_0_13_0(source));
        }
        if self.version.major == 0 && self.version.minor == 12 {
            return Ok(self.build_node_config_after_0_12_0(source));
        }
        Ok(self.build_node_config_before_0_8_19(source))
    }

    fn build_node_config_after_0_13_0(&self, source: &NewestNodeConfig) -> NodeConfig {
        let rng = OsRng;

        let trusted_peers: Vec<TrustedPeer> = source
            .p2p
            .trusted_peers
            .iter()
            .map(|peer| {
                let id = NodeId::from(
                    <chain_crypto::SecretKey<chain_crypto::Ed25519>>::generate(rng).to_public(),
                );

                TrustedPeer {
                    id: Some(id.to_string()),
                    address: peer.address.clone(),
                }
            })
            .collect();

        NodeConfig {
            storage: source.storage.clone(),
            single_log: source.log.clone().map(Into::into),
            log: None,
            rest: Rest {
                listen: source.rest.listen,
                cors: None,
                tls: None,
            },
            jrpc: source.jrpc.clone(),
            p2p: P2p {
                trusted_peers,
                public_address: source.p2p.public_address.clone(),
                listen: None,
                max_inbound_connections: None,
                max_connections: None,
                topics_of_interest: None,
                allow_private_addresses: source.p2p.allow_private_addresses,
                policy: source.p2p.policy.clone(),
                layers: source.p2p.layers.clone(),
                public_id: None,
            },
            mempool: source.mempool.clone(),
            bootstrap_from_trusted_peers: source.bootstrap_from_trusted_peers,
            skip_bootstrap: source.skip_bootstrap,
        }
    }

    fn build_node_config_after_0_12_0(&self, source: &NewestNodeConfig) -> NodeConfig {
        let trusted_peers: Vec<TrustedPeer> = source
            .p2p
            .trusted_peers
            .iter()
            .map(|peer| TrustedPeer {
                id: None,
                address: peer.address.clone(),
            })
            .collect();

        NodeConfig {
            storage: source.storage.clone(),
            single_log: source.log.clone().map(Into::into),
            log: None,
            rest: Rest {
                listen: source.rest.listen,
                cors: None,
                tls: None,
            },
            jrpc: source.jrpc.clone(),
            p2p: P2p {
                trusted_peers,
                public_address: source.p2p.public_address.clone(),
                listen: None,
                max_inbound_connections: None,
                max_connections: None,
                topics_of_interest: None,
                allow_private_addresses: source.p2p.allow_private_addresses,
                policy: source.p2p.policy.clone(),
                layers: source.p2p.layers.clone(),
                public_id: None,
            },
            mempool: source.mempool.clone(),
            bootstrap_from_trusted_peers: source.bootstrap_from_trusted_peers,
            skip_bootstrap: source.skip_bootstrap,
        }
    }

    fn generate_legacy_poldercast_id(rng: &mut OsRng) -> String {
        let mut bytes: [u8; 24] = [0; 24];
        rng.fill_bytes(&mut bytes);
        hex::encode(bytes)
    }

    fn build_node_config_before_0_8_19(&self, source: &NewestNodeConfig) -> NodeConfig {
        let mut rng = OsRng;
        let trusted_peers: Vec<TrustedPeer> = source
            .p2p
            .trusted_peers
            .iter()
            .map(|peer| {
                let id = {
                    if let Some(id) = &peer.id {
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
            log: source.log.clone().map(Into::into),
            single_log: None,
            rest: Rest {
                listen: source.rest.listen,
                cors: None,
                tls: None,
            },
            jrpc: source.jrpc.clone(),
            p2p: P2p {
                trusted_peers,
                public_address: source.p2p.public_address.clone(),
                listen: None,
                max_inbound_connections: None,
                max_connections: None,
                topics_of_interest: source
                    .p2p
                    .layers
                    .as_ref()
                    .and_then(|c| c.topics_of_interest.clone()),
                allow_private_addresses: source.p2p.allow_private_addresses,
                policy: source.p2p.policy.clone(),
                layers: None,
                public_id: None,
            },
            mempool: source.mempool.clone(),
            bootstrap_from_trusted_peers: source.bootstrap_from_trusted_peers,
            skip_bootstrap: source.skip_bootstrap,
        }
    }
}

mod data;

use crate::jormungandr::{
    legacy::{
        config::node::data::{P2p, TrustedPeer},
        version_0_13_0,
    },
    NodeConfigBuilder, Version,
};
pub use data::LegacyNodeConfig;
use jormungandr_lib::interfaces::{
    Log, NodeConfig, NodeId, Rest, TrustedPeer as NewestTrustedPeer,
};
use rand::RngCore;
use rand_core::OsRng;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("unsupported version")]
    UnsupportedVersion(Version),
    #[error(transparent)]
    SerdeYaml(#[from] serde_yaml::Error),
    #[error(transparent)]
    Fixture(#[from] assert_fs::fixture::FixtureError),
}

/// Used to build configuration for legacy nodes.
/// It uses yaml_rust instead of serde yaml serializer
/// beacuse config model is always up to date with newest config schema
/// while legacy node requires na old one
pub struct LegacyNodeConfigBuilder {
    #[allow(dead_code)]
    version: Version,
    config: NodeConfig,
}

impl LegacyNodeConfigBuilder {
    pub fn with_trusted_peers(mut self, trusted_peer: Vec<NewestTrustedPeer>) -> Self {
        self.config.p2p.trusted_peers = trusted_peer;
        self
    }
}

impl Default for LegacyNodeConfigBuilder {
    fn default() -> Self {
        Self {
            version: Version::new(0, 11, 0),
            config: NodeConfigBuilder::default().build(),
        }
    }
}

impl LegacyNodeConfigBuilder {
    pub fn new(version: Version) -> Self {
        Self {
            version,
            ..Default::default()
        }
    }

    pub fn with_log(mut self, log: Log) -> Self {
        self.config.log = Some(log);
        self
    }

    pub fn with_storage(mut self, storage: PathBuf) -> Self {
        self.config.storage = Some(storage);
        self
    }

    pub fn based_on(mut self, config: NodeConfig) -> Self {
        self.config = config;
        self
    }

    pub fn build(self) -> Result<LegacyNodeConfig, Error> {
        LegacyNodeConfigConverter::new(self.version).convert(&self.config)
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
    pub fn convert(&self, source: &NodeConfig) -> Result<LegacyNodeConfig, Error> {
        if self.version >= version_0_13_0() {
            return Ok(self.build_node_config_after_0_13_0(source));
        }
        if self.version.major == 0 && self.version.minor == 12 {
            return Ok(self.build_node_config_after_0_12_0(source));
        }
        Ok(self.build_node_config_before_0_8_19(source))
    }

    fn build_node_config_after_0_13_0(&self, source: &NodeConfig) -> LegacyNodeConfig {
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

        LegacyNodeConfig {
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

    fn build_node_config_after_0_12_0(&self, source: &NodeConfig) -> LegacyNodeConfig {
        let trusted_peers: Vec<TrustedPeer> = source
            .p2p
            .trusted_peers
            .iter()
            .map(|peer| TrustedPeer {
                id: None,
                address: peer.address.clone(),
            })
            .collect();

        LegacyNodeConfig {
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

    fn build_node_config_before_0_8_19(&self, source: &NodeConfig) -> LegacyNodeConfig {
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

        LegacyNodeConfig {
            storage: source.storage.clone(),
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
            single_log: source.log.clone().map(Into::into),
        }
    }
}

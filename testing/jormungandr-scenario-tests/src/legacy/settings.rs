use jormungandr_lib::interfaces::NodeSecret;
use jormungandr_testing_utils::{
    testing::network_builder::{Node as NodeTemplate, NodeSetting},
    testing::{node::configuration::legacy::NodeConfig as LegacyConfig, LegacyNodeConfigConverter},
    Version,
};

#[derive(Debug, Clone)]
pub struct LegacySettings {
    pub alias: String,
    pub config: LegacyConfig,
    pub secret: NodeSecret,
    pub node_topology: NodeTemplate,
}

impl LegacySettings {
    pub fn from_settings(settings: NodeSetting, version: &Version) -> Self {
        let converter = LegacyNodeConfigConverter::new(version.clone());
        LegacySettings {
            alias: settings.alias.clone(),
            config: converter
                .convert(&settings.config)
                .expect("cannot convert node config to legacy"),
            secret: settings.secret().clone(),
            node_topology: settings.node_topology.clone(),
        }
    }

    pub fn secret(&self) -> &NodeSecret {
        &self.secret
    }

    pub fn config(&self) -> &LegacyConfig {
        &self.config
    }
}

use jormungandr_integration_tests::common::legacy::LegacyNodeConfigConverter;
use jormungandr_testing_utils::{
    legacy::NodeConfig as LegacyConfig,
    testing::network_builder::{Node as NodeTemplate,NodeSetting}
};
use jormungandr_lib::interfaces::NodeSecret;
use jormungandr_integration_tests::common::legacy::Version;

#[derive(Debug, Clone)]
pub struct LegacySettings{
    pub alias: String,
    pub config: LegacyConfig,
    pub secret: NodeSecret,
    pub node_topology: NodeTemplate,
}

impl LegacySettings {
    pub fn from_settings(settings: NodeSetting, version: &Version) -> Self {
        let converter = LegacyNodeConfigConverter::new(version.clone());
        LegacySettings{
            alias: settings.alias.clone(),
            config: converter.convert(settings.config.clone()).expect("cannot convert node config to legacy"),
            secret: settings.secrets().clone(),
            node_topology: settings.node_topology.clone(),
        }
    }

    pub fn secrets(&self) -> &NodeSecret {
        &self.secret
    }

    pub fn config(&self) -> &LegacyConfig {
        &self.config
    }
}
use crate::common::{
    configuration::{jormungandr_config::JormungandrConfig, node_config_model::TrustedPeer},
    jormungandr::{JormungandrProcess,Starter,ConfigurationBuilder},
};
use crate::mock::client::JormungandrClient;
use std::{thread, time::Duration};

const LOCALHOST: &str = "127.0.0.1";

pub struct Config {
    host: String,
    port: u16,
}

impl Config {
    pub fn attach_to_local_node(port: u16) -> Self {
        Self {
            host: String::from(LOCALHOST),
            port: port,
        }
    }

    pub fn client(&self) -> JormungandrClient {
        JormungandrClient::new(&self.host, self.port)
    }
}

pub fn bootstrap_node() -> (JormungandrProcess, JormungandrConfig) {
    let config = ConfigurationBuilder::new().with_slot_duration(4).build();
    let server = Starter::new().config(config.clone()).start().unwrap();
    thread::sleep(Duration::from_secs(4));
    (server, config)
}

pub fn build_configuration(mock_port: u16) -> JormungandrConfig {
    let trusted_peer = TrustedPeer {
        address: format!("/ip4/{}/tcp/{}", LOCALHOST, mock_port),
        id: "ed25519_pk1hdhe4mnus0uxaf25gxeryskvwtytlzeuvan8glp3n63ztvv0v78qczpm32".to_owned(),
    };

    ConfigurationBuilder::new()
        .with_slot_duration(4)
        .with_trusted_peers(vec![trusted_peer])
        .build()
}

pub fn bootstrap_node_with_peer(mock_port: u16) -> (JormungandrProcess, JormungandrConfig) {
    let config = build_configuration(mock_port);
    let server = Starter::new().config(config.clone()).start().unwrap();
    thread::sleep(Duration::from_secs(4));
    (server, config)
}

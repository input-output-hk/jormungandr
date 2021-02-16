use crate::common::{
    configuration::JormungandrParams,
    jormungandr::{ConfigurationBuilder, JormungandrProcess, Starter},
};
use chain_impl_mockchain::chaintypes::ConsensusVersion;
use jormungandr_lib::interfaces::TrustedPeer;
use jormungandr_testing_utils::testing::node::grpc::JormungandrClient;

use assert_fs::TempDir;
use std::thread;
use std::time::Duration;

const DEFAULT_SLOT_DURATION: u8 = 4;
const LOCALHOST: &str = "127.0.0.1";

pub struct Config {
    host: String,
    port: u16,
}

impl Config {
    pub fn attach_to_local_node(port: u16) -> Self {
        Self {
            host: String::from(LOCALHOST),
            port,
        }
    }

    pub fn client(&self) -> JormungandrClient {
        JormungandrClient::new(&self.host, self.port)
    }
}

pub struct Fixture {
    temp_dir: TempDir,
    slot_duration: u8,
}

impl Fixture {
    pub fn new(slot_duration: u8) -> Self {
        let temp_dir = TempDir::new().unwrap();
        Fixture {
            temp_dir,
            slot_duration,
        }
    }

    pub fn bootstrap_node(&self) -> (JormungandrProcess, JormungandrParams) {
        let config = ConfigurationBuilder::new()
            .with_slot_duration(self.slot_duration)
            .build(&self.temp_dir);
        let server = Starter::new().config(config.clone()).start_async().unwrap();
        thread::sleep(Duration::from_secs(4));
        (server, config)
    }

    pub fn build_configuration(&self, mock_port: u16) -> JormungandrParams {
        let trusted_peer = TrustedPeer {
            address: format!("/ip4/{}/tcp/{}", LOCALHOST, mock_port)
                .parse()
                .unwrap(),
            id: None,
        };

        ConfigurationBuilder::new()
            .with_slot_duration(4)
            .with_block0_consensus(ConsensusVersion::GenesisPraos)
            .with_trusted_peers(vec![trusted_peer])
            .build(&self.temp_dir)
    }

    pub fn bootstrap_node_with_peer(
        &self,
        mock_port: u16,
    ) -> (JormungandrProcess, JormungandrParams) {
        let config = self.build_configuration(mock_port);
        let server = Starter::new().config(config.clone()).start_async().unwrap();
        thread::sleep(Duration::from_secs(4));
        (server, config)
    }
}

impl Default for Fixture {
    fn default() -> Self {
        Self::new(DEFAULT_SLOT_DURATION)
    }
}

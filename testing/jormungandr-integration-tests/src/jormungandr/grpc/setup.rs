use chain_impl_mockchain::chaintypes::ConsensusVersion;
use jormungandr_lib::interfaces::TrustedPeer;
use jormungandr_testing_utils::testing::{node::grpc::JormungandrClient, SyncNode};

use crate::common::{
    configuration::JormungandrParams,
    jormungandr::{ConfigurationBuilder, JormungandrProcess, Starter},
};
use assert_fs::TempDir;
use std::time::Duration;
const DEFAULT_SLOT_DURATION: u8 = 1;
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

pub mod client {
    use super::*;
    pub struct ClientBootstrap {
        pub client: JormungandrClient,
        pub server: JormungandrProcess,
        pub config: JormungandrParams,
    }

    pub fn default() -> ClientBootstrap {
        bootstrap(
            ConfigurationBuilder::new()
                .with_slot_duration(DEFAULT_SLOT_DURATION)
                .to_owned(),
        )
    }

    pub fn bootstrap(config: ConfigurationBuilder) -> ClientBootstrap {
        let dir = TempDir::new().unwrap();
        let config = config.build(&dir);
        let server = Starter::new()
            .temp_dir(dir)
            .config(config.clone())
            .start_async()
            .unwrap();
        std::thread::sleep(Duration::from_secs(4));
        let client = Config::attach_to_local_node(config.get_p2p_listen_port()).client();
        ClientBootstrap {
            client,
            server,
            config,
        }
    }
}

pub mod server {
    use super::*;
    use crate::common::configuration;
    const SERVER_RETRY_WAIT: Duration = Duration::from_secs(1);
    const TIMEOUT: Duration = Duration::from_secs(60);

    pub struct ServerBootstrap {
        pub server: JormungandrProcess,
        pub config: JormungandrParams,
        pub mock_port: u16,
    }

    pub fn default() -> ServerBootstrap {
        bootstrap(
            configuration::get_available_port(),
            ConfigurationBuilder::new()
                .with_slot_duration(DEFAULT_SLOT_DURATION)
                .with_block0_consensus(ConsensusVersion::GenesisPraos)
                .to_owned(),
        )
    }

    pub fn bootstrap(mock_port: u16, mut config: ConfigurationBuilder) -> ServerBootstrap {
        let dir = TempDir::new().unwrap();
        let trusted_peer = TrustedPeer {
            address: format!("/ip4/{}/tcp/{}", LOCALHOST, mock_port)
                .parse()
                .unwrap(),
            id: None,
        };
        let config = config.with_trusted_peers(vec![trusted_peer]).build(&dir);
        let server = Starter::new()
            .temp_dir(dir)
            .config(config.clone())
            .start_async()
            .unwrap();
        ServerBootstrap {
            server,
            config,
            mock_port,
        }
    }

    impl ServerBootstrap {
        pub fn wait_server_online(&self) {
            let started = std::time::Instant::now();
            loop {
                if self.server.is_running() {
                    return;
                }
                if started.elapsed() > TIMEOUT {
                    println!("{}", self.server.log_content());
                    panic!("Timeout elapsed while waiting for server to go online");
                }
                std::thread::sleep(SERVER_RETRY_WAIT);
            }
        }
    }
}

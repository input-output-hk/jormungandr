use assert_fs::TempDir;
use jormungandr_automation::jormungandr::{
    get_available_port,
    grpc::{client::JormungandrWatchClient, JormungandrClient},
    JormungandrProcess,
};
use jormungandr_lib::interfaces::TrustedPeer;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
const DEFAULT_SLOT_DURATION: u8 = 2;
const LOCALHOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
pub struct Config {
    addr: SocketAddr,
}

impl Config {
    pub fn attach_to_local_node(port: u16) -> Self {
        Self {
            addr: SocketAddr::new(LOCALHOST, port),
        }
    }

    pub fn client(&self) -> JormungandrClient {
        JormungandrClient::new(self.addr)
    }

    pub fn watch_client(&self) -> JormungandrWatchClient {
        JormungandrWatchClient::new(self.addr)
    }
}

pub mod client {
    use super::*;
    use crate::startup::SingleNodeTestBootstrapper;
    use jormungandr_automation::jormungandr::{
        grpc::client::JormungandrWatchClient, Block0ConfigurationBuilder,
    };
    use jormungandr_lib::interfaces::Block0Configuration;

    pub struct ClientBootstrap {
        pub client: JormungandrClient,
        pub watch_client: JormungandrWatchClient,
        pub server: JormungandrProcess,
        pub block0_config: Block0Configuration,
    }

    pub fn default() -> ClientBootstrap {
        bootstrap(
            Block0ConfigurationBuilder::default()
                .with_slot_duration(DEFAULT_SLOT_DURATION.try_into().unwrap()),
        )
    }

    pub fn bootstrap(block0_config: Block0ConfigurationBuilder) -> ClientBootstrap {
        let dir = TempDir::new().unwrap();
        let context = SingleNodeTestBootstrapper::default()
            .with_block0_config(block0_config)
            .as_bft_leader()
            .build();
        let server = context.start_node(dir).unwrap();

        let attached_config = Config::attach_to_local_node(server.p2p_listen_addr().port());
        let client = attached_config.client();
        let watch_client = attached_config.watch_client();
        ClientBootstrap {
            client,
            server,
            block0_config: context.block0_config(),
            watch_client,
        }
    }
}

pub mod server {
    use super::*;
    use crate::{context::TestContext, startup::SingleNodeTestBootstrapper};
    use jormungandr_automation::jormungandr::{Block0ConfigurationBuilder, NodeConfigBuilder};
    use thor::{Block0ConfigurationBuilderExtension, StakePool};

    pub struct ServerBootstrap {
        pub test_context: TestContext,
        pub mock_port: u16,
    }

    pub fn default() -> ServerBootstrap {
        bootstrap(
            get_available_port(),
            Block0ConfigurationBuilder::default()
                .with_slot_duration(DEFAULT_SLOT_DURATION.try_into().unwrap()),
        )
    }

    pub fn bootstrap(mock_port: u16, config: Block0ConfigurationBuilder) -> ServerBootstrap {
        let owner = thor::Wallet::default();
        let stake_pool = StakePool::new(&owner);
        let trusted_peer = TrustedPeer {
            address: format!("/ip4/{}/tcp/{}", LOCALHOST, mock_port)
                .parse()
                .unwrap(),
            id: None,
        };
        let node_config = NodeConfigBuilder::default().with_trusted_peers(vec![trusted_peer]);
        let test_context = SingleNodeTestBootstrapper::default()
            .with_block0_config(
                config
                    .with_wallets_having_some_values(vec![&owner])
                    .with_stake_pool_and_delegation(&stake_pool, vec![&owner])
                    .with_some_consensus_leader(),
            )
            .with_node_config(node_config)
            .as_genesis_praos_stake_pool(&stake_pool)
            .build();

        ServerBootstrap {
            test_context,
            mock_port,
        }
    }
}

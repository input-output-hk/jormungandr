//! REST API of the node

pub mod v0_node_stats;
pub mod v0_utxo;

mod server_service;

pub use self::server_service::{Error, ServerService};

use self::v0_node_stats::StatsCounter;
use blockcfg::mock::Mockchain;
use blockchain::BlockchainR;
use settings::start::{Error as ConfigError, Rest};
use settings::Error as SettingsError;

pub fn start_rest_server(
    config: &Rest,
    stats_counter: StatsCounter,
    blockchain: BlockchainR<Mockchain>,
) -> Result<ServerService, SettingsError> {
    let prefix = config
        .prefix
        .as_ref()
        .map(|prefix| prefix.as_str())
        .unwrap_or("/");
    ServerService::builder(&config.pkcs12, config.listen.clone(), prefix)
        .add_handler(v0_node_stats::crate_handler(stats_counter))
        .add_handler(v0_utxo::crate_handler(blockchain))
        .build()
        .map_err(|e| SettingsError::Start(ConfigError::InvalidRest(e)))
}

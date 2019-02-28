//! REST API of the node

mod server_service;

pub mod v0;

pub use self::server_service::{Error, ServerService};

use self::v0::node::stats::StatsCounter;
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
        .add_handler(v0::node::stats::crate_handler(stats_counter))
        .add_handler(v0::utxo::crate_handler(blockchain))
        .build()
        .map_err(|e| SettingsError::Start(ConfigError::InvalidRest(e)))
}

use crate::scenario::settings::PrepareWalletProxySettings;
use crate::vit_station::VitStationSettings;
use crate::wallet::NodeAlias;
use crate::wallet::NodeSetting;
use crate::Context;
use jormungandr_testing_utils::testing::network_builder::Topology as TopologyTemplate;
pub use jormungandr_testing_utils::testing::network_builder::WalletProxySettings;
use rand::CryptoRng;
use rand::RngCore;
use std::collections::HashMap;
use std::net::SocketAddr;

impl PrepareWalletProxySettings for WalletProxySettings {
    fn prepare<RNG>(
        context: &mut Context<RNG>,
        vit_stations: &HashMap<NodeAlias, VitStationSettings>,
    ) -> Self
    where
        RNG: RngCore + CryptoRng,
    {
        let vit_station_settings = vit_stations
            .values()
            .next()
            .expect("no vit stations defined");

        WalletProxySettings {
            proxy_address: context.generate_new_rest_listen_address(),
            vit_station_address: vit_station_settings.address.clone(),
            node_backend_address: None,
        }
    }
}

use lazy_static::lazy_static;
use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags};
use std::{
    collections::HashSet,
    sync::atomic::{AtomicU16, Ordering},
};

mod block0_config_builder;
mod configuration_builder;
mod jormungandr_config;
mod node_config;
mod node_config_builder;
mod secret_model_factory;

pub use block0_config_builder::Block0ConfigurationBuilder;
pub use configuration_builder::ConfigurationBuilder;
pub use jormungandr_config::JormungandrParams;
pub use node_config::TestConfig;
pub use node_config_builder::NodeConfigBuilder;
pub use secret_model_factory::{write_secret, SecretModelFactory};

lazy_static! {
    static ref NEXT_AVAILABLE_PORT_NUMBER: AtomicU16 = AtomicU16::new(10000);
    static ref OCCUPIED_PORTS: HashSet<u16> = {
        let af_flags = AddressFamilyFlags::IPV4;
        let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;
        get_sockets_info(af_flags, proto_flags)
            .unwrap()
            .into_iter()
            .map(|s| s.local_port())
            .collect::<HashSet<_>>()
    };
}

pub fn get_available_port() -> u16 {
    loop {
        let candidate_port = NEXT_AVAILABLE_PORT_NUMBER.fetch_add(1, Ordering::SeqCst);
        if !(*OCCUPIED_PORTS).contains(&candidate_port) {
            return candidate_port;
        }
    }
}

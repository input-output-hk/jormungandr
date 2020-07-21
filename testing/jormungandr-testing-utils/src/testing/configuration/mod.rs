extern crate lazy_static;
extern crate rand;

use self::lazy_static::lazy_static;
use self::rand::Rng;
use std::sync::atomic::{AtomicU16, Ordering};

mod block0_config_builder;
mod jormungandr_config;
mod legacy;
mod node_config;
mod node_config_builder;
mod secret_model_factory;

pub use block0_config_builder::Block0ConfigurationBuilder;
pub use jormungandr_config::JormungandrParams;
pub use legacy::{LegacyConfigConverter, LegacyConfigConverterError, LegacyNodeConfigConverter};
pub use node_config::TestConfig;
pub use node_config_builder::NodeConfigBuilder;
pub use secret_model_factory::{write_secret, SecretModelFactory};

lazy_static! {
    static ref NEXT_AVAILABLE_PORT_NUMBER: AtomicU16 = {
        let initial_port = rand::thread_rng().gen_range(6000, 10999);
        AtomicU16::new(initial_port)
    };
}

pub fn get_available_port() -> u16 {
    NEXT_AVAILABLE_PORT_NUMBER.fetch_add(1, Ordering::SeqCst)
}

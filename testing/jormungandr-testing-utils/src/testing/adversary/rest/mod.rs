mod handlers;

use crate::testing::{adversary::AdversaryNode, configuration::get_available_port};
use chain_crypto::Ed25519;
use chain_impl_mockchain::{block::Header, testing::data::StakePool};
use jormungandr_lib::crypto::{hash::Hash, key::SigningKey};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use warp::Filter;

pub type State = Arc<Mutex<AdversaryRest>>;

pub struct AdversaryRest {
    address: Option<SocketAddr>,
    adversary: AdversaryNode,
    signing_key: Option<SigningKey<Ed25519>>,
    stake_pool: Option<StakePool>,
}

impl AdversaryRest {
    pub fn new(adversary: AdversaryNode) -> Self {
        Self {
            address: None,
            adversary,
            signing_key: None,
            stake_pool: None,
        }
    }

    pub fn address(self, address: SocketAddr) -> Self {
        Self {
            address: Some(address),
            ..self
        }
    }

    pub fn signing_key(self, key: SigningKey<Ed25519>) -> Self {
        Self {
            signing_key: Some(key),
            ..self
        }
    }

    pub fn stake_pool(self, stake_pool: StakePool) -> Self {
        Self {
            stake_pool: Some(stake_pool),
            ..self
        }
    }

    pub fn start(self) {
        let address = self
            .address
            .unwrap_or_else(|| SocketAddr::new("127.0.0.1".parse().unwrap(), get_available_port()));

        let state = Arc::new(Mutex::new(self));

        let state_filter = warp::any().map(move || state.clone());

        let invalid_fragment = warp::path("invalid_fragment")
            .and(warp::path::end())
            .and(warp::body::json())
            .and(state_filter.clone())
            .map(handlers::invalid_fragment)
            .boxed();

        let invalid_hash = warp::path("invalid_hash")
            .and(warp::path::end())
            .and(warp::body::json())
            .and(state_filter.clone())
            .map(handlers::invalid_hash)
            .boxed();

        let invalid_signature = warp::path("invalid_signature")
            .and(warp::path::end())
            .and(warp::body::json())
            .and(state_filter.clone())
            .map(handlers::invalid_signature)
            .boxed();

        let nonexistent_leader = warp::path::path("nonexistent_leader")
            .and(warp::path::end())
            .and(warp::body::json())
            .and(state_filter.clone())
            .map(handlers::nonexistent_leader)
            .boxed();

        let wrong_leader = warp::path("wrong_leader")
            .and(warp::path::end())
            .and(warp::body::json())
            .and(state_filter)
            .map(handlers::wrong_leader)
            .boxed();

        let route = warp::body::content_length_limit(32 * 1024)
            .and(warp::post())
            .and(
                invalid_fragment
                    .or(invalid_hash)
                    .or(invalid_signature)
                    .or(nonexistent_leader)
                    .or(wrong_leader),
            );

        let rt = tokio::runtime::Builder::new_multi_thread()
            .thread_name("warp runtime")
            .enable_all()
            .build()
            .unwrap();

        println!("Adversary node listening on {}", address);

        rt.block_on(async { warp::serve(route).run(address).await });
    }
}

#[derive(Debug, Deserialize)]
pub struct Request {
    address: SocketAddr,
    #[serde(default)]
    parent: Parent,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Parent {
    Tip,
    Block0,
    Hash(Hash),
}
impl Parent {
    pub fn get_header(&self, adversary: &AdversaryNode) -> Header {
        match self {
            Parent::Block0 => adversary.block0_configuration().to_block().header().clone(),
            Parent::Tip => adversary.node_data().read().unwrap().tip().unwrap(),
            Parent::Hash(hash) => adversary
                .node_data()
                .read()
                .unwrap()
                .get_block(hash.into_hash())
                .unwrap()
                .header()
                .clone(),
        }
    }
}

impl Default for Parent {
    fn default() -> Self {
        Self::Tip
    }
}

use chain_impl_mockchain::block::BlockDate;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use warp::{Filter, Reply};
pub type AdversaryLock = Arc<Mutex<AdversaryNode>>;
use crate::testing::adversary::{block::BlockBuilder, AdversaryNode};
use jormungandr_lib::crypto::hash::Hash;

pub fn start(adversary: AdversaryLock) {
    let adversary_clone = adversary.clone();
    let nonexistent_leader = warp::path::path("nonexistent_leader")
        .and(warp::path::end())
        .and(warp::body::json())
        .and(warp::any().map(move || adversary_clone.clone()))
        .map(handle_nonexistent_leader)
        .boxed();

    let adversary_clone = adversary.clone();
    let incorrect_signature = warp::path("incorrect_signature")
        .and(warp::path::end())
        .and(warp::any().map(move || adversary_clone.clone()))
        .map(incorrect_signature)
        .boxed();

    let adversary_clone = adversary.clone();
    let wrong_leader = warp::path("wrong_leader")
        .and(warp::path::end())
        .and(warp::any().map(move || adversary_clone.clone()))
        .map(wrong_leader)
        .boxed();

    let adversary_clone = adversary.clone();
    let invalid_fragment = warp::path("invalid_fragment")
        .and(warp::path::end())
        .and(warp::any().map(move || adversary_clone.clone()))
        .map(invalid_fragment)
        .boxed();

    let incorrect_hash = warp::path("incorrect_hash")
        .and(warp::path::end())
        .and(warp::any().map(move || adversary.clone()))
        .map(incorrect_hash)
        .boxed();

    let route = warp::body::content_length_limit(1024 * 32)
        .and(warp::post())
        .and(
            nonexistent_leader
                .or(incorrect_signature)
                .or(wrong_leader)
                .or(invalid_fragment)
                .or(incorrect_hash),
        );

    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_name("warp runtime")
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        warp::serve(route)
            .run("127.0.0.1:18210".parse::<SocketAddr>().unwrap())
            .await
    });
}

fn incorrect_signature(_: AdversaryLock) -> impl Reply {
    warp::reply::with_status("", StatusCode::NOT_IMPLEMENTED)
}

fn wrong_leader(_: AdversaryLock) -> impl Reply {
    warp::reply::with_status("", StatusCode::NOT_IMPLEMENTED)
}

fn invalid_fragment(_: AdversaryLock) -> impl Reply {
    warp::reply::with_status("", StatusCode::NOT_IMPLEMENTED)
}

fn incorrect_hash(_: AdversaryLock) -> impl Reply {
    warp::reply::with_status("", StatusCode::NOT_IMPLEMENTED)
}

fn handle_nonexistent_leader(
    request: NonexistentLeaderRequest,
    adversary_lock: AdversaryLock,
) -> impl Reply {
    let NonexistentLeaderRequest { address, parent } = request;

    let block = if let Ok(adversary) = adversary_lock.lock() {
        let parent = match parent {
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
        };

        let parent_block_date = parent.block_date();

        let block0_config = adversary.block0_configuration().blockchain_configuration;

        let slots_per_epoch: u32 = block0_config.slots_per_epoch.into();

        let block_date = BlockDate {
            epoch: parent_block_date.epoch + (parent_block_date.slot_id + 1) / slots_per_epoch,
            slot_id: (parent_block_date.slot_id + 1) % slots_per_epoch,
        };

        BlockBuilder::bft(block_date, parent).build()
    } else {
        panic!("Mutex poisoned");
    };

    // Separate thread since `JormungandrClient` will spawn a new tokio runtime
    std::thread::spawn(move || {
        adversary_lock
            .lock()
            .unwrap()
            .send_block_to_peer(address, block)
            .map(|_| warp::reply::with_status(String::new(), StatusCode::OK))
            .unwrap_or_else(|e| warp::reply::with_status(e.to_string(), StatusCode::BAD_REQUEST))
    })
    .join()
    .unwrap()
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NonexistentLeaderRequest {
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

impl Default for Parent {
    fn default() -> Self {
        Self::Tip
    }
}

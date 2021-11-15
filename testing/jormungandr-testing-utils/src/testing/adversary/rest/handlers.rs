use crate::testing::{adversary::block::BlockBuilder, startup, FragmentBuilder};
use chain_impl_mockchain::{
    block::{Block, BlockDate, ContentsBuilder},
    chaintypes::ConsensusType,
};
use reqwest::StatusCode;
use std::net::SocketAddr;
use warp::{reply::WithStatus, Reply};

use super::{Request, State};

pub(super) fn invalid_signature(request: Request, state: State) -> impl Reply {
    let Request { address, parent } = request;

    let block = if let Ok(state) = state.lock() {
        let block0_config = state
            .adversary
            .block0_configuration()
            .blockchain_configuration;

        let parent = parent.get_header(&state.adversary);

        let parent_block_date = parent.block_date();

        let slots_per_epoch: u32 = block0_config.slots_per_epoch.into();

        let block_date = BlockDate {
            epoch: parent_block_date.epoch + (parent_block_date.slot_id + 1) / slots_per_epoch,
            slot_id: (parent_block_date.slot_id + 1) % slots_per_epoch,
        };

        match block0_config.block0_consensus {
            ConsensusType::Bft => {
                if let Some(key) = state.signing_key.clone() {
                    BlockBuilder::bft(block_date, parent)
                        .signing_key(key)
                        .invalid_signature()
                        .build()
                } else {
                    return warp::reply::with_status(
                        String::from("No signing key available to sign block"),
                        StatusCode::FORBIDDEN,
                    );
                }
            }
            ConsensusType::GenesisPraos => {
                if let Some(stake_pool) = state.stake_pool.clone() {
                    BlockBuilder::genesis_praos(block_date, parent)
                        .stake_pool(stake_pool)
                        .invalid_signature()
                        .build()
                } else {
                    return warp::reply::with_status(
                        String::from("No stake pool available to sign block"),
                        StatusCode::FORBIDDEN,
                    );
                }
            }
        }
    } else {
        panic!("Mutex poisoned");
    };

    send_block(state, address, block)
}

pub(super) fn wrong_leader(_: Request, _: State) -> impl Reply {
    warp::reply::with_status("", StatusCode::NOT_IMPLEMENTED)
}

pub(super) fn invalid_fragment(request: Request, state: State) -> impl Reply {
    let Request { address, parent } = request;

    let block = if let Ok(state) = state.lock() {
        let block0_config = state
            .adversary
            .block0_configuration()
            .blockchain_configuration;

        let parent = parent.get_header(&state.adversary);

        let parent_block_date = parent.block_date();

        let slots_per_epoch: u32 = block0_config.slots_per_epoch.into();

        let block_date = BlockDate {
            epoch: parent_block_date.epoch + (parent_block_date.slot_id + 1) / slots_per_epoch,
            slot_id: (parent_block_date.slot_id + 1) % slots_per_epoch,
        };

        let mut contents_builder = ContentsBuilder::default();

        contents_builder.push(
            FragmentBuilder::new(
                &state
                    .adversary
                    .block0_configuration()
                    .to_block()
                    .header()
                    .id()
                    .into(),
                &block0_config.linear_fees,
                BlockDate::first().next_epoch(),
            )
            .transaction(
                &startup::create_new_account_address(),
                startup::create_new_account_address().address(),
                42.into(),
            )
            .unwrap(),
        );

        match block0_config.block0_consensus {
            ConsensusType::Bft => {
                if let Some(key) = state.signing_key.clone() {
                    BlockBuilder::bft(block_date, parent)
                        .contents(contents_builder.into())
                        .signing_key(key)
                        .build()
                } else {
                    return warp::reply::with_status(
                        String::from("No signing key available to sign block"),
                        StatusCode::FORBIDDEN,
                    );
                }
            }
            ConsensusType::GenesisPraos => {
                if let Some(stake_pool) = state.stake_pool.clone() {
                    BlockBuilder::genesis_praos(block_date, parent)
                        .contents(contents_builder.into())
                        .stake_pool(stake_pool)
                        .build()
                } else {
                    return warp::reply::with_status(
                        String::from("No stake pool available to sign block"),
                        StatusCode::FORBIDDEN,
                    );
                }
            }
        }
    } else {
        panic!("Mutex poisoned");
    };

    send_block(state, address, block)
}

pub(super) fn invalid_hash(request: Request, state: State) -> impl Reply {
    let Request { address, parent } = request;

    let block = if let Ok(state) = state.lock() {
        let block0_config = state
            .adversary
            .block0_configuration()
            .blockchain_configuration;

        let parent = parent.get_header(&state.adversary);

        let parent_block_date = parent.block_date();

        let slots_per_epoch: u32 = block0_config.slots_per_epoch.into();

        let block_date = BlockDate {
            epoch: parent_block_date.epoch + (parent_block_date.slot_id + 1) / slots_per_epoch,
            slot_id: (parent_block_date.slot_id + 1) % slots_per_epoch,
        };

        match block0_config.block0_consensus {
            ConsensusType::Bft => {
                if let Some(key) = state.signing_key.clone() {
                    BlockBuilder::bft(block_date, parent)
                        .signing_key(key)
                        .invalid_hash()
                        .build()
                } else {
                    return warp::reply::with_status(
                        String::from("No signing key available to sign block"),
                        StatusCode::FORBIDDEN,
                    );
                }
            }
            ConsensusType::GenesisPraos => {
                if let Some(stake_pool) = state.stake_pool.clone() {
                    BlockBuilder::genesis_praos(block_date, parent)
                        .stake_pool(stake_pool)
                        .invalid_hash()
                        .build()
                } else {
                    return warp::reply::with_status(
                        String::from("No stake pool available to sign block"),
                        StatusCode::FORBIDDEN,
                    );
                }
            }
        }
    } else {
        panic!("Mutex poisoned");
    };

    send_block(state, address, block)
}

pub(super) fn nonexistent_leader(request: Request, state: State) -> impl Reply {
    let Request { address, parent } = request;

    let block = if let Ok(state) = state.lock() {
        let parent = parent.get_header(&state.adversary);

        let parent_block_date = parent.block_date();

        let block0_config = state
            .adversary
            .block0_configuration()
            .blockchain_configuration;

        let slots_per_epoch: u32 = block0_config.slots_per_epoch.into();

        let block_date = BlockDate {
            epoch: parent_block_date.epoch + (parent_block_date.slot_id + 1) / slots_per_epoch,
            slot_id: (parent_block_date.slot_id + 1) % slots_per_epoch,
        };

        match block0_config.block0_consensus {
            ConsensusType::Bft => BlockBuilder::bft(block_date, parent).build(),
            ConsensusType::GenesisPraos => BlockBuilder::genesis_praos(block_date, parent).build(),
        }
    } else {
        panic!("Mutex poisoned");
    };

    send_block(state, address, block)
}

fn send_block(state: State, address: SocketAddr, block: Block) -> WithStatus<String> {
    // Separate thread since `JormungandrClient` will spawn a new tokio runtime
    std::thread::spawn(move || {
        warp::reply::with_status(
            state
                .lock()
                .expect("Mutex poisoned")
                .adversary
                .send_block_to_peer(address, block)
                .map(|_| String::new())
                .unwrap_or_else(|e| e.to_string()),
            StatusCode::OK,
        )
    })
    .join()
    .unwrap()
}

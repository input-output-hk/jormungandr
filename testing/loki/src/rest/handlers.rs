use super::{Context, Request};
use crate::block::BlockBuilder;
use chain_impl_mockchain::{
    block::{Block, BlockDate, ContentsBuilder},
    chaintypes::ConsensusType,
};
use reqwest::StatusCode;
use std::net::SocketAddr;
use thor::FragmentBuilder;
use warp::{reply::WithStatus, Reply};

pub(super) fn invalid_signature(request: Request, context: Context) -> impl Reply {
    let Request { address, parent } = request;

    let block = if let Ok(context) = context.lock() {
        let block0_config = context
            .adversary
            .block0_configuration()
            .blockchain_configuration;

        let parent = parent.get_header(&context.adversary);

        let parent_block_date = parent.block_date();

        let slots_per_epoch: u32 = block0_config.slots_per_epoch.into();

        let block_date = BlockDate {
            epoch: parent_block_date.epoch + (parent_block_date.slot_id + 1) / slots_per_epoch,
            slot_id: (parent_block_date.slot_id + 1) % slots_per_epoch,
        };

        match block0_config.block0_consensus {
            ConsensusType::Bft => {
                if let Some(key) = context.signing_key.clone() {
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
                if let Some(stake_pool) = context.stake_pool.clone() {
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

    send_block(context, address, block)
}

pub(super) fn wrong_leader(_: Request, _: Context) -> impl Reply {
    // TODO: Implement
    warp::reply::with_status("", StatusCode::NOT_IMPLEMENTED)
}

pub(super) fn invalid_fragment(request: Request, context: Context) -> impl Reply {
    let Request { address, parent } = request;

    let block = if let Ok(context) = context.lock() {
        let block0_config = context
            .adversary
            .block0_configuration()
            .blockchain_configuration;

        let parent = parent.get_header(&context.adversary);

        let parent_block_date = parent.block_date();

        let slots_per_epoch: u32 = block0_config.slots_per_epoch.into();

        let block_date = BlockDate {
            epoch: parent_block_date.epoch + (parent_block_date.slot_id + 1) / slots_per_epoch,
            slot_id: (parent_block_date.slot_id + 1) % slots_per_epoch,
        };

        let mut contents_builder = ContentsBuilder::default();

        contents_builder.push(
            FragmentBuilder::new(
                &context
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
                &thor::Wallet::default(),
                thor::Wallet::default().address(),
                42.into(),
            )
            .unwrap(),
        );

        match block0_config.block0_consensus {
            ConsensusType::Bft => {
                if let Some(key) = context.signing_key.clone() {
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
                if let Some(stake_pool) = context.stake_pool.clone() {
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

    send_block(context, address, block)
}

pub(super) fn invalid_hash(request: Request, context: Context) -> impl Reply {
    let Request { address, parent } = request;

    let block = if let Ok(context) = context.lock() {
        let block0_config = context
            .adversary
            .block0_configuration()
            .blockchain_configuration;

        let parent = parent.get_header(&context.adversary);

        let parent_block_date = parent.block_date();

        let slots_per_epoch: u32 = block0_config.slots_per_epoch.into();

        let block_date = BlockDate {
            epoch: parent_block_date.epoch + (parent_block_date.slot_id + 1) / slots_per_epoch,
            slot_id: (parent_block_date.slot_id + 1) % slots_per_epoch,
        };

        match block0_config.block0_consensus {
            ConsensusType::Bft => {
                if let Some(key) = context.signing_key.clone() {
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
                if let Some(stake_pool) = context.stake_pool.clone() {
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

    send_block(context, address, block)
}

pub(super) fn nonexistent_leader(request: Request, context: Context) -> impl Reply {
    let Request { address, parent } = request;

    let block = if let Ok(context) = context.lock() {
        let parent = parent.get_header(&context.adversary);

        let parent_block_date = parent.block_date();

        let block0_config = context
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

    send_block(context, address, block)
}

fn send_block(context: Context, address: SocketAddr, block: Block) -> WithStatus<String> {
    // Separate thread since `JormungandrClient` will spawn a new tokio runtime
    std::thread::spawn(move || {
        warp::reply::with_status(
            context
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

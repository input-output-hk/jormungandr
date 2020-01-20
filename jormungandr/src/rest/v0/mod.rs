mod handlers;

use actix_web::{
    dev::HttpServiceFactory,
    web::{delete, get, post, resource, scope},
};

pub fn service(root_path: &str) -> impl HttpServiceFactory {
    scope(root_path)
        .route(
            "/account/{account_id}",
            get().to(handlers::get_account_state),
        )
        .route("/block/{block_id}", get().to(handlers::get_block_id))
        .route(
            "/block/{block_id}/next_id",
            get().to(handlers::get_block_next_id),
        )
        .route("/fragment/logs", get().to(handlers::get_message_logs))
        .service(
            resource("/leaders")
                .route(get().to(handlers::get_leaders))
                .route(post().to(handlers::post_leaders)),
        )
        .route("/leaders/logs", get().to(handlers::get_leaders_logs))
        .route(
            "/leaders/{leader_id}",
            delete().to(handlers::delete_leaders),
        )
        .route("/network/stats", get().to(handlers::get_network_stats))
        .route(
            "/network/p2p/quarantined",
            get().to(handlers::get_network_p2p_quarantined),
        )
        .route(
            "/network/p2p/non_public",
            get().to(handlers::get_network_p2p_non_public),
        )
        .route(
            "/network/p2p/available",
            get().to(handlers::get_network_p2p_available),
        )
        .route(
            "/network/p2p/view",
            get().to(handlers::get_network_p2p_view),
        )
        .route(
            "/network/p2p/view/{topic}",
            get().to(handlers::get_network_p2p_view_topic),
        )
        .route("/settings", get().to(handlers::get_settings))
        .route("/stake", get().to(handlers::get_stake_distribution))
        .route("/stake_pools", get().to(handlers::get_stake_pools))
        .route("/stake_pool/{pool_id}", get().to(handlers::get_stake_pool))
        .route("/shutdown", get().to(handlers::get_shutdown))
        .route("/message", post().to(handlers::post_message))
        .route("/node/stats", get().to(handlers::get_stats_counter))
        .route("/tip", get().to(handlers::get_tip))
        .route(
            "/utxo/{fragment_id}/{output_index}",
            get().to(handlers::get_utxo),
        )
        .route("/diagnostic", get().to(handlers::get_diagnostic))
}

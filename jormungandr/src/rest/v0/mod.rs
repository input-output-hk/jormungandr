mod handlers;

use actix_web::dev::Resource;

pub fn resources() -> Vec<(
    &'static str,
    &'static dyn Fn(&mut Resource<handlers::Context>),
)> {
    vec![
        ("/account/{account_id}", &|r| {
            r.get().with_async(handlers::get_account_state)
        }),
        ("/block/{block_id}", &|r| {
            r.get().with_async(handlers::get_block_id)
        }),
        ("/block/{block_id}/next_id", &|r| {
            r.get().with_async(handlers::get_block_next_id)
        }),
        ("/fragment/logs", &|r| {
            r.get().with_async(handlers::get_message_logs)
        }),
        ("/leaders", &|r| {
            r.get().with(handlers::get_leaders);
            r.post().with(handlers::post_leaders);
        }),
        ("/leaders/logs", &|r| {
            r.get().with_async(handlers::get_leaders_logs);
        }),
        ("/leaders/{leader_id}", &|r| {
            r.delete().with(handlers::delete_leaders)
        }),
        ("/network/stats", &|r| {
            r.get().with_async(handlers::get_network_stats)
        }),
        ("/network/p2p/quarantined", &|r| {
            r.get().with_async(handlers::get_network_p2p_quarantined)
        }),
        ("/network/p2p/non_public", &|r| {
            r.get().with_async(handlers::get_network_p2p_non_public)
        }),
        ("/network/p2p/available", &|r| {
            r.get().with_async(handlers::get_network_p2p_available)
        }),
        ("/network/p2p/view", &|r| {
            r.get().with_async(handlers::get_network_p2p_view)
        }),
        ("/network/p2p/view/{topic}", &|r| {
            r.get().with_async(handlers::get_network_p2p_view_topic)
        }),
        ("/settings", &|r| r.get().with_async(handlers::get_settings)),
        ("/stake", &|r| {
            r.get().with_async(handlers::get_stake_distribution)
        }),
        ("/stake_pools", &|r| {
            r.get().with_async(handlers::get_stake_pools)
        }),
        ("/stake_pool/{pool_id}", &|r| {
            r.get().with_async(handlers::get_stake_pool)
        }),
        ("/shutdown", &|r| r.get().with(handlers::get_shutdown)),
        ("/message", &|r| r.post().with(handlers::post_message)),
        ("/node/stats", &|r| {
            r.get().with_async(handlers::get_stats_counter)
        }),
        ("/tip", &|r| r.get().with_async(handlers::get_tip)),
        ("/utxo/{fragment_id}/{output_index}", &|r| {
            r.get().with_async(handlers::get_utxo)
        }),
        ("/diagnostic", &|r| r.get().with(handlers::get_diagnostic)),
    ]
}

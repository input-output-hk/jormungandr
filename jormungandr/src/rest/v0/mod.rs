mod handlers;

use actix_web::App;

pub fn app(context: handlers::Context) -> App<handlers::Context> {
    App::with_state(context)
        .prefix("/api/v0")
        .resource("/account/{account_id}", |r| {
            r.get().with_async(handlers::get_account_state)
        })
        .resource("/block/{block_id}", |r| {
            r.get().with_async(handlers::get_block_id)
        })
        .resource("/block/{block_id}/next_id", |r| {
            r.get().with_async(handlers::get_block_next_id)
        })
        .resource("/fragment/logs", |r| {
            r.get().with_async(handlers::get_message_logs)
        })
        .resource("/leaders", |r| {
            r.get().with(handlers::get_leaders);
            r.post().with(handlers::post_leaders);
        })
        .resource("/leaders/logs", |r| {
            r.get().with_async(handlers::get_leaders_logs);
        })
        .resource("/leaders/{leader_id}", |r| {
            r.delete().with(handlers::delete_leaders)
        })
        .resource("/settings", |r| r.get().with_async(handlers::get_settings))
        .resource("/stake", |r| {
            r.get().with_async(handlers::get_stake_distribution)
        })
        .resource("/stake_pools", |r| {
            r.get().with_async(handlers::get_stake_pools)
        })
        .resource("/shutdown", |r| r.get().with(handlers::get_shutdown))
        .resource("/message", |r| r.post().a(handlers::post_message))
        .resource("/node/stats", |r| {
            r.get().with_async(handlers::get_stats_counter)
        })
        .resource("/tip", |r| r.get().with_async(handlers::get_tip))
        .resource("/utxo", |r| r.get().with_async(handlers::get_utxos))
}

mod handlers;
pub mod logic;

use crate::rest::{display_internal_server_error, ContextLock};
use warp::{http::StatusCode, Filter, Rejection, Reply};

pub fn filter(
    context: ContextLock,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let with_context = warp::any().map(move || context.clone());
    let root = warp::path!("v0" / ..);

    #[cfg(feature = "evm")]
    let address_mapping = {
        let root = warp::path!("address_mapping" / ..);

        let get_jor_address = warp::path!("jormungandr_address" / String)
            .and(warp::get())
            .and(with_context.clone())
            .and_then(handlers::get_jor_address)
            .boxed();

        let get_evm_address = warp::path!("evm_address" / String)
            .and(warp::get())
            .and(with_context.clone())
            .and_then(handlers::get_evm_address)
            .boxed();

        root.and(get_jor_address.or(get_evm_address)).boxed()
    };

    let shutdown = warp::path!("shutdown")
        .and(warp::get().or(warp::post()))
        .and(with_context.clone())
        .and_then(|_, context| handlers::shutdown(context))
        .boxed();

    let account = warp::path!("account" / String)
        .and(warp::get())
        .and(with_context.clone())
        .and_then(handlers::get_account_state)
        .boxed();

    let block = {
        let root = warp::path!("block" / ..);

        let get = warp::path!(String)
            .and(warp::get())
            .and(with_context.clone())
            .and_then(handlers::get_block_id)
            .boxed();

        let get_next = warp::path!(String / "next_id")
            .and(warp::get())
            .and(warp::query())
            .and(with_context.clone())
            .and_then(handlers::get_block_next_id)
            .boxed();

        root.and(get.or(get_next)).boxed()
    };

    let fragment = {
        let root = warp::path!("fragment" / ..).boxed();

        let logs = warp::path!("logs")
            .and(warp::get())
            .and(with_context.clone())
            .and_then(handlers::get_message_logs)
            .boxed();

        root.and(logs).boxed()
    };

    let leaders = {
        let root = warp::path!("leaders" / ..).boxed();

        let logs = warp::path!("logs")
            .and(warp::get())
            .and(with_context.clone())
            .and_then(handlers::get_leaders_logs)
            .boxed();

        root.and(logs).boxed()
    };

    let p2p = {
        let root = warp::path!("p2p" / ..);

        let quarantined = warp::path!("quarantined")
            .and(warp::get())
            .and(with_context.clone())
            .and_then(handlers::get_network_p2p_quarantined)
            .boxed();

        let non_public = warp::path!("non_public")
            .and(warp::get())
            .and(with_context.clone())
            .and_then(handlers::get_network_p2p_non_public)
            .boxed();

        let available = warp::path!("available")
            .and(warp::get())
            .and(with_context.clone())
            .and_then(handlers::get_network_p2p_available)
            .boxed();

        let view = {
            let root = warp::path!("view" / ..);

            let view = warp::path::end()
                .and(warp::get())
                .and(with_context.clone())
                .and_then(handlers::get_network_p2p_view)
                .boxed();

            let view_topic = warp::path!(String)
                .and(warp::get())
                .and(with_context.clone())
                .and_then(handlers::get_network_p2p_view_topic)
                .boxed();

            root.and(view.or(view_topic)).boxed()
        };

        root.and(quarantined.or(non_public).or(available).or(view))
            .boxed()
    };

    let network = {
        let root = warp::path!("network" / ..);

        let stats = warp::path!("stats")
            .and(warp::get())
            .and(with_context.clone())
            .and_then(handlers::get_network_stats)
            .boxed();

        root.and(stats.or(p2p)).boxed()
    };

    let settings = warp::path!("settings")
        .and(warp::get())
        .and(with_context.clone())
        .and_then(handlers::get_settings)
        .boxed();

    let stake = {
        let root = warp::path!("stake" / ..);

        let get = warp::path::end()
            .and(warp::get())
            .and(with_context.clone())
            .and_then(handlers::get_stake_distribution)
            .boxed();

        let get_at = warp::path!(u32)
            .and(warp::get())
            .and(with_context.clone())
            .and_then(handlers::get_stake_distribution_at)
            .boxed();

        root.and(get.or(get_at)).boxed()
    };

    let stake_pools = warp::path!("stake_pools")
        .and(warp::get())
        .and(with_context.clone())
        .and_then(handlers::get_stake_pools)
        .boxed();

    let stake_pool = warp::path!("stake_pool" / String)
        .and(warp::get())
        .and(with_context.clone())
        .and_then(handlers::get_stake_pool)
        .boxed();

    let message = warp::path!("message")
        .and(warp::post())
        .and(warp::body::bytes())
        .and(with_context.clone())
        .and_then(handlers::post_message)
        .boxed();

    let node_stats = warp::path!("node" / "stats")
        .and(warp::get())
        .and(with_context.clone())
        .and_then(handlers::get_stats_counter)
        .boxed();

    let tip = warp::path!("tip")
        .and(warp::get())
        .and(with_context.clone())
        .and_then(handlers::get_tip)
        .boxed();

    let rewards = {
        let root = warp::path!("rewards" / ..);

        let history = warp::path!("history" / usize)
            .and(warp::get())
            .and(with_context.clone())
            .and_then(handlers::get_rewards_info_history)
            .boxed();

        let epoch = warp::path!("epoch" / u32)
            .and(warp::get())
            .and(with_context.clone())
            .and_then(handlers::get_rewards_info_epoch)
            .boxed();

        let remaining = warp::path!("remaining")
            .and(warp::get())
            .and(with_context.clone())
            .and_then(handlers::get_rewards_remaining)
            .boxed();

        root.and(history.or(epoch).or(remaining)).boxed()
    };

    let utxo = warp::path!("utxo" / String / u8)
        .and(warp::get())
        .and(with_context.clone())
        .and_then(handlers::get_utxo)
        .boxed();

    let diagnostic = warp::path!("diagnostic")
        .and(warp::get())
        .and(with_context.clone())
        .and_then(handlers::get_diagnostic)
        .boxed();

    let votes = {
        let root = warp::path!("vote" / "active" / ..);
        let committees = warp::path!("committees")
            .and(warp::get())
            .and(with_context.clone())
            .and_then(handlers::get_committees)
            .boxed();

        let vote_plans = warp::path!("plans")
            .and(warp::get())
            .and(with_context)
            .and_then(handlers::get_active_vote_plans)
            .boxed();
        root.and(committees.or(vote_plans)).boxed()
    };

    let routes = shutdown
        .or(account)
        .or(block)
        .or(fragment)
        .or(leaders)
        .or(network)
        .or(settings)
        .or(stake)
        .or(stake_pools)
        .or(stake_pool)
        .or(message)
        .or(node_stats)
        .or(tip)
        .or(rewards)
        .or(utxo)
        .or(diagnostic)
        .or(votes);

    #[cfg(feature = "evm")]
    let routes = routes.or(address_mapping);

    root.and(routes.boxed()).recover(handle_rejection).boxed()
}

/// Convert rejections to actual HTTP errors
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(err) = err.find::<logic::Error>() {
        let (body, code) = match err {
            logic::Error::PublicKey(_) | logic::Error::Hash(_) | logic::Error::Hex(_) => {
                (err.to_string(), StatusCode::BAD_REQUEST)
            }
            logic::Error::Fragment(summary) => (
                serde_json::to_string(&summary).unwrap(),
                StatusCode::BAD_REQUEST,
            ),
            err => (
                display_internal_server_error(err),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        };

        return Ok(warp::reply::with_status(body, code));
    }

    Err(err)
}

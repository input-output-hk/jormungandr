use std::net::SocketAddr;
use warp::Filter as _;
use warp_json_rpc::Builder;

pub struct Config {
    pub listen: SocketAddr,
}

pub async fn start_rpc_server(config: Config) {
    // it is initial dummy impl just for initialization rpc instance
    let method = warp_json_rpc::filters::json_rpc()
        .and(warp_json_rpc::filters::method("dummy"))
        .and(warp_json_rpc::filters::params::<(isize, isize)>())
        .map(|res: Builder, (lhs, rhs)| res.success(lhs + rhs).unwrap());

    warp::serve(method).bind(config.listen).await;
}

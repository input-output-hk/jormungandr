use futures::future;
use std::{convert::Infallible, net::SocketAddr};
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

    let service = warp_json_rpc::service(method);
    let serivce_fn =
        hyper::service::make_service_fn(move |_| future::ok::<_, Infallible>(service.clone()));

    hyper::Server::bind(&config.listen)
        .serve(serivce_fn)
        .await
        .unwrap();
}

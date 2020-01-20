mod handlers;

use actix_web::{
    dev::HttpServiceFactory,
    web::{get, post, scope},
};

pub fn service(root_path: &str) -> impl HttpServiceFactory {
    scope(root_path)
        .route("/graphql", post().to(handlers::graphql))
        .route("/graphiql", get().to(handlers::graphiql))
}

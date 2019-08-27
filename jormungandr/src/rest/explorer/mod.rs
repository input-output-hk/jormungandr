mod handlers;

use actix_web::dev::Resource;

pub fn resources() -> Vec<(
    &'static str,
    &'static dyn Fn(&mut Resource<handlers::Context>),
)> {
    vec![
        ("/graphql", &|r| r.post().with_async(handlers::graphql)),
        ("/graphiql", &|r| r.get().with(handlers::graphiql)),
    ]
}

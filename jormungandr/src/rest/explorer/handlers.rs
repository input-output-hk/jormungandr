use actix_web::error::{ErrorBadRequest, ErrorInternalServerError, ErrorServiceUnavailable};
use actix_web::{http, Json, Responder, State};
use actix_web::{Error, HttpResponse};

use futures::{Future, IntoFuture};

use crate::explorer::graphql::GraphQLRequest;
pub use crate::rest::Context;

macro_rules! ActixFuture {
    () => { impl Future<Item = impl Responder + 'static, Error = impl Into<Error> + 'static> + 'static }
}

pub fn graphiql(_context: State<Context>) -> impl Responder {
    let html = juniper::http::graphiql::graphiql_source("/explorer/graphql");
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

pub fn graphql(context: State<Context>, data: Json<GraphQLRequest>) -> ActixFuture!() {
    context
        .try_full_fut()
        .and_then(|context| {
            context
                .explorer
                .clone()
                .ok_or(ErrorServiceUnavailable("Explorer not enabled"))
        })
        .and_then(move |explorer| {
            // Run the query in a threadpool, as Juniper is synchronous
            actix_threadpool::run(move || {
                Some(data.execute(&explorer.schema, &explorer.context()))
                    .filter(|ref response| response.is_ok())
                    .ok_or(ErrorBadRequest("Error processing query"))
                    .and_then(|ref res| Ok(serde_json::to_string(res)?))
            })
            .map_err(|err| ErrorInternalServerError(err))
        })
        .map(|response| {
            HttpResponse::Ok()
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(response)
        })
        .map_err(|err| ErrorInternalServerError(err))
}

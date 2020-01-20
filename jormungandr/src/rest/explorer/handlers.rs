use actix_threadpool::BlockingError;
use actix_web::error::{ErrorBadRequest, ErrorInternalServerError, ErrorServiceUnavailable};
use actix_web::web::{Data, Json};
use actix_web::{http, Error, HttpResponse, Responder};

use futures::Future;

use crate::explorer::graphql::GraphQLRequest;
pub use crate::rest::Context;

macro_rules! ActixFuture {
    () => { impl Future<Item = impl Responder + 'static, Error = impl Into<Error> + 'static> + 'static }
}

pub fn graphiql(_context: Data<Context>) -> impl Responder {
    let html = juniper::http::graphiql::graphiql_source("/explorer/graphql");
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

pub fn graphql(context: Data<Context>, data: Json<GraphQLRequest>) -> ActixFuture!() {
    context
        .try_full_fut()
        .and_then(|context| {
            context
                .explorer
                .clone()
                .ok_or(ErrorServiceUnavailable("Explorer not enabled"))
        })
        .and_then(move |explorer|
            // Run the query in a threadpool, as Juniper is synchronous
            actix_threadpool::run(move || {
                let response = data.execute(&explorer.schema, &explorer.context());
                match response.is_ok() {
                    true => serde_json::to_string(&response).map(Some),
                    false => Ok(None),
                }})
            .then(|res| match res {
                Ok(Some(response)) => Ok(response),
                Ok(None) => Err(ErrorBadRequest("Error processing query")),
                Err(BlockingError::Canceled) => Err(ErrorInternalServerError("Data execution cancelled")),
                Err(BlockingError::Error(serde_err)) => Err(ErrorInternalServerError(serde_err)),
            }
        ))
        .map(|response| {
            HttpResponse::Ok()
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(response)
        })
        .map_err(|err| ErrorInternalServerError(err))
}

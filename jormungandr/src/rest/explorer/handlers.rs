use actix_threadpool::BlockingError;
use actix_web::error::{ErrorBadRequest, ErrorInternalServerError, ErrorServiceUnavailable};
use actix_web::web::{Data, Json};
use actix_web::{http, Error, HttpResponse, Responder};

use crate::explorer::graphql::GraphQLRequest;
pub use crate::rest::Context;

pub async fn graphiql() -> impl Responder {
    let html = juniper::http::graphiql::graphiql_source("/explorer/graphql");
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

pub async fn graphql(
    context: Data<Context>,
    data: Json<GraphQLRequest>,
) -> Result<impl Responder, Error> {
    let explorer = context
        .try_full()
        .await?
        .explorer
        .clone()
        .ok_or(ErrorServiceUnavailable("Explorer not enabled"))?;
    // Run the query in a threadpool, as Juniper is synchronous
    let res = actix_threadpool::run(move || {
        let response = data.execute(&explorer.schema, &explorer.context());
        match response.is_ok() {
            true => serde_json::to_string(&response).map(Some),
            false => Ok(None),
        }
    })
    .await;
    let response = match res {
        Ok(Some(response)) => response,
        Ok(None) => return Err(ErrorBadRequest("Error processing query")),
        Err(BlockingError::Canceled) => {
            return Err(ErrorInternalServerError("Data execution cancelled"))
        }
        Err(BlockingError::Error(serde_err)) => return Err(ErrorInternalServerError(serde_err)),
    };
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(response))
}

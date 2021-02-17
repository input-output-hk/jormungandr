use crate::rest::{context, display_internal_server_error, ContextLock};
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use thiserror::Error;
use warp::reject::Reject;
use warp::{http::Response as HttpResponse, http::StatusCode, Filter, Rejection, Reply};

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum ExplorerGraphQLError {
    #[error(transparent)]
    Context(#[from] context::Error),
}

impl Reject for ExplorerGraphQLError {}

pub fn filter(
    context: ContextLock,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let root = warp::path!("explorer" / ..);

    let context_filter_check = std::sync::Arc::clone(&context);

    let with_full_context = warp::any()
        .map(move || context_filter_check.clone())
        .and_then(|ctx: ContextLock| async move {
            ctx.read()
                .await
                .try_full()
                .map_err(ExplorerGraphQLError::Context)
                .map_err(warp::reject::custom)
                .map(|_| ())
        });

    let schema = async_graphql::Schema::build(
        crate::explorer::graphql::Query {},
        async_graphql::EmptyMutation,
        async_graphql::EmptySubscription,
    )
    .data(EContext { context })
    .finish();

    let graphql_post = with_full_context
        .and(async_graphql_warp::graphql(schema))
        .and_then(|_, (schema, request)| handler(schema, request));

    let graphql = warp::path!("graphql").and(graphql_post).boxed();

    let graphql_playground = warp::path::end().and(warp::get()).map(|| {
        HttpResponse::builder()
            .header("content-type", "text/html")
            .body(playground_source(
                GraphQLPlaygroundConfig::new("/").subscription_endpoint("/"),
            ))
    });

    let graphiql = warp::path!("graphiql").and(graphql_playground).boxed();

    root.and(graphql.or(graphiql)).recover(handle_rejection)
}

pub async fn handler(
    schema: async_graphql::Schema<
        crate::explorer::graphql::Query,
        async_graphql::EmptyMutation,
        async_graphql::EmptySubscription,
    >,
    request: async_graphql::Request,
) -> Result<impl Reply, std::convert::Infallible> {
    Ok::<_, std::convert::Infallible>(async_graphql_warp::Response::from(
        schema.execute(request).await,
    ))
}

/// Convert rejections to actual HTTP errors
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(err) = err.find::<ExplorerGraphQLError>() {
        let (body, code) = (
            display_internal_server_error(err),
            StatusCode::INTERNAL_SERVER_ERROR,
        );

        return Ok(warp::reply::with_status(body, code));
    }

    Err(err)
}

pub(crate) struct EContext {
    context: ContextLock,
}

impl EContext {
    pub(crate) async fn get(
        &self,
    ) -> Result<crate::explorer::graphql::EContext, ExplorerGraphQLError> {
        self.context
            .read()
            .await
            .try_full()
            .map_err(ExplorerGraphQLError::Context)
            .map(|ctx| ctx.explorer.clone().unwrap().context())
    }
}

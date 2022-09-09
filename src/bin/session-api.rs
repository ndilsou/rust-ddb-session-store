use std::env;

use aws_sdk_dynamodb::Client;
use ddb_session_store::{
    alb::AlbRouter,
    api,
    utils::{setup_sdk_config, setup_tracing}, store::SessionStore,
};
use http::Method;
use lambda_http::{service_fn, Request};
use tracing::{info, instrument};

type E = Box<dyn std::error::Error + Sync + Send + 'static>;

#[instrument]
#[tokio::main]
async fn main() -> Result<(), E> {
    setup_tracing();

    let config = setup_sdk_config().await;
    let ddb = Client::new(&config);
    let store = SessionStore::new(
        &ddb,
        env::var("TABLE_NAME")
            .to_owned()
            .expect("TABLE_NAME must be set"),
    );

    let mut router = AlbRouter::new();
    router.insert(Method::GET, "/sessions", |r| api::get_session(&store, r))?;
    router.insert(Method::POST, "/sessions", |r| {
        api::create_session(&store, r)
    })?;
    router.insert(Method::DELETE, "/sessions/:user_id", |r| {
        api::delete_user_sessions(&store, r)
    })?;

    lambda_http::run(service_fn(|request: Request| router.handle(request))).await?;
    info!("execution started");

    Ok(())
}

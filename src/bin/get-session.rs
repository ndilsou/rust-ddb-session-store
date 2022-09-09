use std::env;

use aws_sdk_dynamodb::Client;
use ddb_session_store::{utils::{setup_sdk_config, setup_tracing}, api, store::SessionStore};
use lambda_http::{service_fn, Request};
use tracing::{info, instrument};

type E = Box<dyn std::error::Error + Sync + Send + 'static>;

#[instrument]
#[tokio::main]
async fn main() -> Result<(), E> {
    setup_tracing();

    let config = setup_sdk_config().await;
    let ddb = Client::new(&config);
    let  store = SessionStore::new(&ddb, env::var("TABLE_NAME").to_owned().expect("TABLE_NAME must be set"));
    lambda_http::run(service_fn(|event: Request| api::get_session(&store, event))).await?;
    info!("execution started");
    
    Ok(())
}

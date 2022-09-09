use std::{env, collections::HashSet};


use aws_sdk_dynamodb::Client;
use ddb_session_store::{utils::{setup_sdk_config, setup_tracing}, api, store::SessionStore};
use lambda_http::{service_fn, Request};
use tracing::{info, instrument};
use lazy_static::lazy_static;

type E = Box<dyn std::error::Error + Sync + Send + 'static>;

lazy_static! {
    static ref PASSWORDS: HashSet<String> = HashSet::from([
        "pingpong".to_owned(),
        "moultipass".to_owned(),
        "devoid of meaning".to_owned(),
        "perlimpinpin".to_owned()
    ]);
}

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
    lambda_http::run(service_fn(|event: Request| api::create_session(&store, event))).await?;
    info!("execution started");

    Ok(())
}
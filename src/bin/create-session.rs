use std::{env, collections::HashSet};

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::Client;
use ddb_session_store::utils::{response, setup_tracing, SessionStore};
use http::StatusCode;
use lambda_http::{service_fn, IntoResponse, Request};
use serde::Deserialize;
use serde_json::json;
use tracing::{info, instrument, warn};
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

    let region_provider = RegionProviderChain::default_provider().or_else("eu-west-1");
    let config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&config);
    let store = SessionStore::new(
        &client,
        env::var("TABLE_NAME")
            .to_owned()
            .expect("TABLE_NAME must be set"),
    );
    lambda_http::run(service_fn(|event: Request| handler(&store, event))).await?;
    info!("execution started");

    Ok(())
}

#[instrument(skip(store))]
pub async fn handler(store: &SessionStore<'_>, event: Request) -> Result<impl IntoResponse, E> {
    let is_json_content_type = event
        .headers()
        .get(http::header::CONTENT_TYPE)
        .map(|ct| {
            ct.to_str()
                .unwrap_or("")
                .starts_with("application/json")
        })
        .unwrap_or(false);

    if !is_json_content_type {
            return Ok(response(
                StatusCode::BAD_REQUEST,
                json!({ "error": "expects JSON payload" }).to_string(),
            ));
    }

    let req = match serde_json::from_slice::<CreateSessionRequest>(event.body()) {
        Ok(payload) => payload,
        Err(err) => {
            warn!("{}", err.to_string());
            return Ok(response(
                StatusCode::BAD_REQUEST,
                json!({ "error": "invalid payload, cannot parse JSON" }).to_string(),
            ));
        }
    };

    if  !PASSWORDS.contains(&req.password) {
            return Ok(response(
                StatusCode::UNAUTHORIZED,
                json!({ "error": "incorrect password or username" }).to_string(),
            ));
    }

    let session_id = match store.create(req.username).await {
        Ok(session_id) => session_id,
        Err(err) => {
            return Ok(response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": err.to_string() }).to_string(),
            ))
        }
    };

    Ok(response(
        StatusCode::OK,
        json!({
            "sessionId": session_id,
        })
        .to_string(),
    ))
}

#[derive(Debug, Deserialize)]
struct CreateSessionRequest {
    username: String,
    password: String,
}

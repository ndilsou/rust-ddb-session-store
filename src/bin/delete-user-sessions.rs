use std::env;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::Client;
use ddb_session_store::utils::{response, setup_tracing, SessionStore};
use http::StatusCode;
use lambda_http::{service_fn, IntoResponse, Request, RequestExt};
use serde_json::json;
use tracing::{info, instrument};

type E = Box<dyn std::error::Error + Sync + Send + 'static>;

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
    let session_id = match event
        .headers()
        .get(http::header::AUTHORIZATION)
        .map(|h| {
            let session_id = h
                .to_str()
                .unwrap_or("")
                .to_lowercase()
                .replace("bearer ", "");

            if session_id.is_empty() {
                return None;
            }
            Some(session_id)
        })
        .flatten()
    {
        Some(session_id) => session_id,
        None => {
            return Ok(response(
                StatusCode::BAD_REQUEST,
                json!({ "error": "Missing Header: ".to_owned() }).to_string(),
            ))
        }
    };

    let session = match store.get(session_id.to_owned()).await {
        Ok(session) => session,
        Err(err) => {
            return Ok(response(
                StatusCode::UNAUTHORIZED,
                json!({ "error": err.to_string() }).to_string(),
            ))
        }
    };

    if session.username != event.path_parameters().first("username").unwrap() {
        return Ok(response(
            StatusCode::UNAUTHORIZED,
            json!({ "error": "Invalid session" }).to_string(),
        ));
    }

    if let Err(e) = store.delete_user_sessions(session.username.clone()).await {
        return Ok(response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": e.to_string() }).to_string(),
        ));
    }

    Ok(response(
        StatusCode::OK,
        json!({
            "username": session.username,
        })
        .to_string(),
    ))
}

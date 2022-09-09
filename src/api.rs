use std::{collections::HashSet};

use crate::utils::{response};
use crate::store::{SessionStore};
use http::StatusCode;
use lambda_http::{Request, RequestExt, Response};
use lazy_static::lazy_static;
use serde::Deserialize;
use serde_json::json;
use tracing::{info, instrument, warn};

type E = Box<dyn std::error::Error + Sync + Send + 'static>;

lazy_static! {
    static ref PASSWORDS: HashSet<String> = HashSet::from([
        "pingpong".to_owned(),
        "moultipass".to_owned(),
        "devoid of meaning".to_owned(),
        "perlimpinpin".to_owned()
    ]);
}

#[instrument(skip(store))]
pub async fn create_session(
    store: &SessionStore<'_>,
    event: Request,
) -> Result<Response<String>, E> {
    let is_json_content_type = event
        .headers()
        .get(http::header::CONTENT_TYPE)
        .map(|ct| ct.to_str().unwrap_or("").starts_with("application/json"))
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

    if !PASSWORDS.contains(&req.password) {
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

#[instrument(skip(store))]
pub async fn delete_user_sessions(
    store: &SessionStore<'_>,
    event: Request,
) -> Result<Response<String>, E> {
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

#[instrument(skip(store))]
pub async fn get_session(store: &SessionStore<'_>, event: Request) -> Result<Response<String>, E> {
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

    info!("sessionId: {}", session_id);

    let session = match store.get(session_id.to_owned()).await {
        Ok(session) => session,
        Err(err) => {
            return Ok(response(
                StatusCode::UNAUTHORIZED,
                json!({ "error": err.to_string() }).to_string(),
            ))
        }
    };

    Ok(response(
        StatusCode::OK,
        json!({
            "username": session.username,
        })
        .to_string(),
    ))
}

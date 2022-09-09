use std::time;

use aws_config::{meta::region::RegionProviderChain, SdkConfig};
use aws_smithy_types::{timeout, tristate::TriState};
use lambda_http::{http::StatusCode, Response};

pub fn setup_tracing() {
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("failed to set tracing subscriber");
}

pub async fn setup_sdk_config() -> SdkConfig {
    let region_provider = RegionProviderChain::default_provider().or_else("eu-west-1");
    let timeout_config = aws_config::timeout::Config::new()
        .with_api_timeouts(
            timeout::Api::new()
                .with_call_timeout(TriState::Set(time::Duration::from_secs(2)))
                .with_call_attempt_timeout(TriState::Set(time::Duration::from_secs(2))),
        )
        .with_http_timeouts(
            timeout::Http::new()
                .with_read_timeout(TriState::Set(time::Duration::from_secs(2)))
                .with_connect_timeout(TriState::Set(time::Duration::from_secs(2))),
        );

    aws_config::from_env()
        .region(region_provider)
        .timeout_config(timeout_config)
        .load()
        .await
}

pub fn response(status_code: StatusCode, body: String) -> Response<String> {
    Response::builder()
        .status(status_code)
        .header("Content-Type", "application/json")
        .body(body)
        .unwrap()
}

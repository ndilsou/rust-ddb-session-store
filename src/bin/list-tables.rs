use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::Client;
use ddb_session_store::utils::{setup_tracing, response};
use http::StatusCode;
use lambda_http::{service_fn, IntoResponse, Request};
use serde_json::json;
use tracing::{instrument, info};

type E = Box<dyn std::error::Error + Sync + Send + 'static>;

#[tokio::main]
async fn main() -> Result<(), E> {
    setup_tracing();

    let region_provider = RegionProviderChain::default_provider().or_else("eu-west-1");
    let config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&config);

    lambda_http::run(service_fn(|event: Request| get_session(&client, event))).await?;
    Ok(())
}

#[instrument]
pub async fn get_session(ddb: &Client, event: Request) -> Result<impl IntoResponse, E> {
    info!("list-tables called");
    
    let resp = ddb
        .list_tables()
        .send()
        .await
        .map(|resp| resp.table_names().unwrap_or_default().to_vec())
        .map_err(|err| {
            let body = json!({
                "error": err.to_string()
            })
            .to_string();

            body
        });

    let tables = match resp {
        Ok(t) => t,
        Err(body) => return Ok(response(StatusCode::INTERNAL_SERVER_ERROR, body)),
    };

    let body = json!({
        "tables": serde_json::to_string(&tables).unwrap_or_else(|_| {"[]".to_owned()})
    })
    .to_string();
    // // If the event doesn't contain a product ID, we return a 400 Bad Request.
    // let path_parameters = event.path_parameters();
    // let id = match path_parameters.first("id") {
    //     Some(id) => id,
    //     None => {
    //         warn!("Missing 'id' parameter in path");
    //         return Ok(response(
    //             StatusCode::BAD_REQUEST,
    //             json!({ "message": "Missing 'id' parameter in path" }).to_string(),
    //         ));
    //     }
    // };

    Ok(response(StatusCode::OK, body))
}

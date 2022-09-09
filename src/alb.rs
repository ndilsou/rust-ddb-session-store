use std::{collections::HashMap, future::Future};

use futures::{future::LocalBoxFuture, FutureExt};
use http::{Method, Response, StatusCode};
use lambda_http::{Request, RequestExt};
use matchit::{InsertError, Router};
use serde_json::json;
use tracing::info;

use crate::utils::response;

pub type E = Box<dyn std::error::Error + Sync + Send + 'static>;

pub type HandlerResponse = Result<Response<String>, E>;

/// represents for a functional ALB request handler.
pub type RequestHandler = fn(Request) -> Result<Response<String>, E>;
pub type BoxedHandler<'a> = Box<dyn Fn(Request) -> LocalBoxFuture<'a, HandlerResponse> + 'a>;

/// specialises the matchit.Router to work with ALB lambda targets.
pub struct AlbRouter<'b> {
    routers: HashMap<Method, Router<BoxedHandler<'b>>>,
    not_found: RequestHandler,
}

impl<'c> AlbRouter<'c> {
    pub fn new() -> Self {
        AlbRouter::new_with_default(not_found)
    }

    pub fn new_with_default(not_found: RequestHandler) -> Self {
        let routers: HashMap<Method, Router<BoxedHandler>> = HashMap::from([
            (Method::CONNECT, Router::new()),
            (Method::DELETE, Router::new()),
            (Method::GET, Router::new()),
            (Method::HEAD, Router::new()),
            (Method::OPTIONS, Router::new()),
            (Method::PATCH, Router::new()),
            (Method::POST, Router::new()),
            (Method::PUT, Router::new()),
            (Method::TRACE, Router::new()),
        ]);

        AlbRouter { routers, not_found }
    }

    pub fn insert<F, Fut>(
        &mut self,
        method: Method,
        route: impl Into<String>,
        handler: F,
    ) -> Result<(), InsertError>
    where
        F: 'c + (Fn(Request) -> Fut),
        Fut: 'c + Future<Output = HandlerResponse>,
    {
        let router = self
            .routers
            .get_mut(&method)
            .ok_or(InsertError::UnnamedParam)?;

        router.insert(
            route,
            Box::new(move |request| handler(request).boxed_local()),
        )
    }

    pub async fn handle(&self, request: Request) -> HandlerResponse {
        info!(
            "uri: {}, raw_http_path: {}",
            request.uri(),
            request.raw_http_path()
        );
        info!("path_parameters: {:?}", request.path_parameters());

        let not_found_handler = &self.not_found;
        let router = match self.routers.get(request.method()) {
            Some(router) => router,
            None => return not_found_handler(request),
        };

        info!("router found for method {}", request.method());
        let raw_path = request.raw_http_path();
        info!("raw path for matching {}", raw_path);
        let matched = match router.at(&raw_path) {
            Ok(matched) => matched,
            Err(_) => return not_found_handler(request),
        };

        info!("match found!");
        let handler = matched.value;

        let iter = matched
            .params
            .iter()
            .map(|(key, val)| (key.to_owned(), vec![val.to_owned()]));

        let params = HashMap::from_iter(iter);
        let event = request.with_path_parameters(params);

        handler(event).await
    }
}

pub fn not_found(event: Request) -> Result<Response<String>, E> {
    let body = json!({
        "error": format!("endpoint {} not found", event.raw_http_path()),
    })
    .to_string();
    Ok(response(StatusCode::NOT_FOUND, body))
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;

    async fn dummy_handler_a(_: Request) -> HandlerResponse {
        Ok(response(StatusCode::NOT_FOUND, String::from("{}")))
    }

    async fn dummy_handler_b(_: Request) -> HandlerResponse {
        Ok(response(StatusCode::OK, String::from("{}")))
    }

    #[test]
    fn router_should_accept_functions() {
        let mut router = AlbRouter::new();
        let res_a = router.insert(Method::GET, "/tests", dummy_handler_a);
        let res_b = router.insert(Method::GET, "/wow", dummy_handler_b);
        assert!(res_a.is_ok() && res_b.is_ok());
    }

    #[test]
    fn router_should_accept_closures() {
        let mut router = AlbRouter::new();
        let res_a = router.insert(Method::GET, "/tests", |req| dummy_handler_a(req));
        let res_b = router.insert(Method::GET, "/wow", |req| dummy_handler_b(req));
        assert!(res_a.is_ok() && res_b.is_ok());
    }

    #[test]
    fn router_should_accept_closure_with_capture() {
        let map = Rc::new(HashMap::from([("key1", 1), ("key2", 2)]));

        let mut router = AlbRouter::new();

        let mut map_ref = map.clone();
        let res_a = router.insert(Method::GET, "/tests", move |req| {
            let val = *map_ref
                .get("key1")
                .expect("this value should exist and be 1");
            assert!(val == 1);

            dummy_handler_a(req)
        });

        map_ref = map.clone();
        let res_b = router.insert(Method::GET, "/wow", move |req| {
            let val = *map_ref
                .get("key2")
                .expect("this value should exist and be 2");
            assert!(val == 2);

            dummy_handler_b(req)
        });
        assert!(res_a.is_ok() && res_b.is_ok());
    }
}

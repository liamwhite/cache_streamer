use std::sync::Arc;

use axum::extract::{Path, Request, State};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use http::{Method, StatusCode};
use server::Server;

mod aws;
mod request;
mod response;
mod server;
mod sparse;
mod transient_cache;

const TRANSIENT_CACHE_SIZE: usize = 2_000_000_000;
const MAX_LENGTH_FOR_CACHED_OBJECTS: usize = 100_000_000;

#[tokio::main]
async fn main() {
    env_logger::init();

    let state = Arc::new(Server::new(
        TRANSIENT_CACHE_SIZE,
        MAX_LENGTH_FOR_CACHED_OBJECTS,
    ));
    let app = Router::new()
        .route("/", get(root).head(root))
        .route("/*path", get(service).head(service))
        .fallback(client_error);

    let app = app.with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    log::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn root(req: Request) -> impl IntoResponse {
    error(req, StatusCode::NOT_FOUND)
}

async fn client_error(req: Request) -> impl IntoResponse {
    error(req, StatusCode::BAD_REQUEST)
}

async fn service(service: State<Arc<Server>>, Path(path): Path<String>, req: Request) -> Response {
    log::debug!("{} /{}", req.method().as_str(), path);
    // TODO range
    service
        .stream_response(req.method(), &path, &None)
        .await
        .unwrap_or_else(|| error(req, StatusCode::INTERNAL_SERVER_ERROR).into_response())
}

fn error(req: Request, code: StatusCode) -> impl IntoResponse {
    (
        code,
        if req.method() == Method::HEAD {
            ""
        } else {
            code.canonical_reason().unwrap_or("Unknown Error")
        },
    )
}

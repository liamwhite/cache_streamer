use axum::{
    body::Body,
    extract::{Path, Request, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use cache_streamer_http::{HTTPRequestBackend, HTTPService, Url};
use std::sync::Arc;
use tokio::net::TcpListener;

const CACHE_SIZE: usize = 2_000_000_000;
const MAX_LENGTH_FOR_CACHED_OBJECTS: usize = 100_000_000;

fn main() {
    env_logger::init();

    server();
}

#[tokio::main]
async fn server() {
    let base_url = "http://example.com".parse::<Url>().unwrap();
    let backend = HTTPRequestBackend::new(base_url, MAX_LENGTH_FOR_CACHED_OBJECTS);
    let service = HTTPService::new(backend, CACHE_SIZE);

    let app = Router::new()
        .route("/", get(root).head(root))
        .route("/*path", get(call).head(call))
        .fallback(client_error);

    let app = app.with_state(Arc::new(service));
    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn root(req: Request) -> impl IntoResponse {
    error(&req, StatusCode::NOT_FOUND)
}

async fn client_error(req: Request) -> impl IntoResponse {
    error(&req, StatusCode::BAD_REQUEST)
}

fn error(req: &Request, status: StatusCode) -> impl IntoResponse {
    (
        status,
        if req.method() == Method::HEAD {
            ""
        } else {
            status.canonical_reason().unwrap_or("Unknown Error")
        },
    )
}

async fn call(
    service: State<Arc<HTTPService>>,
    Path(path): Path<String>,
    req: Request,
) -> impl IntoResponse {
    let (status, headers, body) = service
        .call(req.method(), &path, req.headers())
        .await
        .into_parts();

    (status, headers, Response::new(Body::from_stream(body)))
}

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

use crate::Config;

#[tokio::main]
pub async fn run(config: &Config) {
    let base_url = config.url.parse::<Url>().unwrap();
    let backend = HTTPRequestBackend::new(base_url, config.limit);
    let service = HTTPService::new(backend, config.capacity);

    let app = Router::new()
        .route("/", get(root).head(root))
        .route("/*path", get(call).head(call))
        .fallback(client_error);

    let app = app.with_state(Arc::new(service));
    let listener = TcpListener::bind(&config.bind_address).await.unwrap();

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

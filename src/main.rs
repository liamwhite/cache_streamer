use std::sync::Arc;

pub use async_trait::async_trait;
use axum::extract::{Path, Request, State};
pub use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use container::TransientCache;
use headers::{HeaderMapExt, Range as RangeHeader};
pub use http::{Method, StatusCode};
use request::{PlainBackend, Range};
use server::Server;

mod aws;
mod container;
mod request;
mod response;
mod server;

const TRANSIENT_CACHE_SIZE: usize = 2_000_000_000;
const MAX_LENGTH_FOR_CACHED_OBJECTS: usize = 100_000_000;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() {
    env_logger::init();

    let backend = Arc::new(PlainBackend::create("https://example.com").unwrap());
    let cache = TransientCache::new(TRANSIENT_CACHE_SIZE);
    let state = Arc::new(Server::new(backend, cache, MAX_LENGTH_FOR_CACHED_OBJECTS));
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
    error(&req, StatusCode::NOT_FOUND)
}

async fn client_error(req: Request) -> impl IntoResponse {
    error(&req, StatusCode::BAD_REQUEST)
}

async fn service(service: State<Arc<Server>>, Path(path): Path<String>, req: Request) -> Response {
    log::debug!("{} /{}", req.method().as_str(), path);

    let range: Option<RangeHeader> = req.headers().typed_get::<RangeHeader>();
    let request_range = match get_single_range(&range) {
        Ok(range) => range,
        Err(..) => return error(&req, StatusCode::RANGE_NOT_SATISFIABLE).into_response(),
    };

    service
        .stream_response(req.method(), &path, &request_range)
        .await
        .unwrap_or_else(|| error(&req, StatusCode::INTERNAL_SERVER_ERROR).into_response())
}

fn error(req: &Request, code: StatusCode) -> impl IntoResponse {
    (
        code,
        if req.method() == Method::HEAD {
            ""
        } else {
            code.canonical_reason().unwrap_or("Unknown Error")
        },
    )
}

fn get_single_range(range: &Option<RangeHeader>) -> Result<Range, ()> {
    match range {
        None => Ok(Range::default()),
        Some(range) => range.try_into(),
    }
}

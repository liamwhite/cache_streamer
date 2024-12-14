use std::sync::Arc;

use bytes::Bytes;
use cache_streamer_lib::types::{BodyStream, RequestRange, ServiceStatus};
use cache_streamer_lib::Service;
use chrono::Utc;
use futures::stream;
use http::{HeaderMap, Method, StatusCode};

use crate::http_request_backend::HTTPRequestBackend;
use crate::http_response::HTTPResponse;
use crate::parse::get_request_range;

/// `cache_streamer` service implementation which makes HTTP requests and returns HTTP responses.
pub struct HTTPService {
    service: Service<String, HTTPResponse>,
}

impl HTTPService {
    /// Builds a new [`HTTPService`].
    ///
    /// `cache_capacity` is the total size of the cache, such as 32GiB.
    ///
    /// The maximum size of individual cacheable responses can be tuned when constructing
    /// the `backend` parameter.
    pub fn new(backend: HTTPRequestBackend, cache_capacity: usize) -> Self {
        let backend = Arc::new(backend);
        let service = Service::new(backend, cache_capacity);

        Self { service }
    }

    /// Fetch a [`HTTPResponse`] corresponding to the given request parameters.
    ///
    /// The output [`HTTPResponse`] is suitable for returning to a client.
    /// All errors are internally handled.
    pub async fn call(&self, method: &Method, key: &String, headers: &HeaderMap) -> HTTPResponse {
        match fetch_into_status(&self.service, method, key, headers).await {
            Ok(response) => erase_body_if_head(response, method),
            Err(status) => synthesize_response(status, method),
        }
    }
}

/// Create a [`BodyStream`] object that returns the contents of the input string.
fn static_body(contents: &'static str) -> BodyStream {
    Box::pin(stream::once(async move { Ok(Bytes::from(contents)) }))
}

/// Remove the body of a [`HTTPResponse`] if the input method is HTTP `HEAD`.
fn erase_body_if_head(mut response: HTTPResponse, method: &Method) -> HTTPResponse {
    if matches!(*method, Method::HEAD) {
        response.set_body(static_body(""));
    }

    response
}

/// Create a new [`HTTPResponse`] where the body is either the canonical name of the status
/// code if the method is not HTTP `HEAD`, or empty if the method is HTTP `HEAD`.
fn synthesize_response(status: StatusCode, method: &Method) -> HTTPResponse {
    let headers = HeaderMap::new();
    let body = if matches!(*method, Method::HEAD) {
        static_body("")
    } else {
        static_body(status.canonical_reason().unwrap_or("Unknown Error"))
    };

    HTTPResponse::new(status, headers, body)
}

/// Using the given [`Service`], fetch a [`HTTPResponse`] corresponding to the given request parameters
/// or return a HTTP [`StatusCode`] indicating an error in processing.
///
/// Currently, the error status which will be returned are:
/// * [`StatusCode::METHOD_NOT_ALLOWED`] when the method is not HTTP `GET` or `HEAD`
/// * [`StatusCode::RANGE_NOT_SATISFIABLE`] when there is an issue with the input range
/// * [`StatusCode::INTERNAL_SERVER_ERROR`] when connecting to the upstream server returns an error
async fn fetch_into_status(
    service: &Service<String, HTTPResponse>,
    method: &Method,
    key: &String,
    headers: &HeaderMap,
) -> Result<HTTPResponse, StatusCode> {
    // We can't handle methods other than GET or HEAD.
    if !matches!(*method, Method::GET | Method::HEAD) {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    }

    let range = get_request_range(headers).ok_or(StatusCode::RANGE_NOT_SATISFIABLE)?;

    // Fetch current time as close as possible to the service call.
    let timepoint = Utc::now();

    // Map errors in the service call to HTTP 500.
    let service_status = service
        .call(&timepoint, key, &range)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Return and don't post-process passed-through responses.
    let mut response = match service_status {
        ServiceStatus::Cache(r) => r,
        ServiceStatus::Passthrough(r) => return Ok(r),
    };

    // Handling the 204 No Content case is not required.
    // However, we must handle 206 Partial Content.
    if matches!(range, RequestRange::None) {
        response.set_status(StatusCode::OK);
    } else {
        response.set_status(StatusCode::PARTIAL_CONTENT);
    };

    Ok(response)
}

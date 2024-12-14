use std::sync::Arc;

use bytes::Bytes;
use cache_streamer_lib::types::{RequestRange, ServiceStatus};
use cache_streamer_lib::Service;
use chrono::Utc;
use futures::stream;
use http::{HeaderMap, Method, StatusCode};

use crate::http_request_backend::HTTPRequestBackend;
use crate::http_response::HTTPResponse;
use crate::parse::get_request_range;

pub struct HTTPService {
    service: Service<String, HTTPResponse>,
}

impl HTTPService {
    pub fn new(backend: HTTPRequestBackend, cache_capacity: usize) -> Self {
        let backend = Arc::new(backend);
        let service = Service::new(backend, cache_capacity);

        Self { service }
    }

    pub async fn call(
        &self,
        method: &Method,
        key: String,
        headers: &HeaderMap,
    ) -> Result<HTTPResponse, StatusCode> {
        // We can't handle methods other than GET or HEAD.
        if !matches!(*method, Method::GET | Method::HEAD) {
            return Err(StatusCode::METHOD_NOT_ALLOWED);
        }

        let range = get_request_range(headers)?;

        // Fetch current time as close as possible to the service call.
        let timepoint = Utc::now();

        // Map errors in the service call to HTTP 500.
        let service_status = self
            .service
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

        // Remove the body from HEAD requests.
        if matches!(*method, Method::HEAD) {
            let body = Box::pin(stream::once(async { Ok(Bytes::new()) }));
            response.set_body(body);
        }

        Ok(response)
    }
}

use std::sync::Arc;

use bytes::Bytes;
use cache_streamer_lib::types::{RequestRange, Response};
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
        Self {
            service: Service::new(Arc::new(backend), cache_capacity),
        }
    }

    pub async fn call(
        &self,
        method: &Method,
        key: String,
        headers: &HeaderMap,
    ) -> Result<HTTPResponse, StatusCode> {
        // We can't handle methods other than GET or HEAD.
        match *method {
            Method::GET | Method::HEAD => {}
            _ => return Err(StatusCode::METHOD_NOT_ALLOWED),
        };

        let range = get_request_range(headers)?;

        // Fetch current time as close as possible to the service call.
        let timepoint = Utc::now();
        let (status, headers, body) = self
            .service
            .call(&timepoint, key, &range)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .into_parts();

        // Return immediately on error status.
        // TODO we don't actually know that this is an error
        if !status.is_success() {
            return Ok(HTTPResponse::from_parts((status, headers), None, body));
        }

        // Figure out what code we want to return with.
        //
        // Handling the 204 No Content case is not required.
        // However, we must handle 206 Partial Content.
        let status = if matches!(range, RequestRange::None) {
            StatusCode::OK
        } else {
            StatusCode::PARTIAL_CONTENT
        };

        // Remove the body from HEAD requests.
        match *method {
            Method::HEAD => {
                let body = stream::once(async { Ok(Bytes::new()) });

                Ok(HTTPResponse::from_parts(
                    (status, headers),
                    None,
                    Box::pin(body),
                ))
            }
            _ => Ok(HTTPResponse::from_parts((status, headers), None, body)),
        }
    }
}

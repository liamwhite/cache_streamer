use std::sync::Arc;

use crate::{Method, Response, StatusCode};
use parking_lot::Mutex;

use crate::container::TransientCache;
use crate::request::{Backend, Range};
use crate::response::{fetch, CacheReader, FetchResponse};
use response::{error_response, passthrough_response, reader_response};

mod header;
mod response;

pub struct Server {
    // NOTE: this must be Arc because the cache item lifetimes
    // need to be independent of this server
    backend: Arc<dyn Backend>,
    cache: Mutex<TransientCache<Arc<CacheReader>>>,
    max_length_for_cached_objects: usize,
}

impl Server {
    pub fn new(
        backend: Arc<dyn Backend>,
        cache: TransientCache<Arc<CacheReader>>,
        max_length_for_cached_objects: usize,
    ) -> Self {
        Self {
            backend,
            cache: Mutex::new(cache),
            max_length_for_cached_objects,
        }
    }

    pub async fn stream_response(
        &self,
        method: &Method,
        path: &str,
        request_range: &Range,
    ) -> Option<Response> {
        // Check to see if we can handle this request.
        if !matches!(*method, Method::GET | Method::HEAD) {
            return Some(error_response(StatusCode::METHOD_NOT_ALLOWED));
        }

        // Try to get the item from cache.
        //
        // The cache may also contain partial items which have not finished streaming yet.
        // This is fine because it will stream any available data to each new client, and
        // once caught up, stream data as it is received from the upstream.
        if let Some(reader) = self.cache.lock().get(path) {
            return reader_response(method, request_range, &reader);
        }

        // The item was not in the cache, so make a request.
        match fetch(
            &self.backend,
            method,
            path,
            request_range,
            self.max_length_for_cached_objects,
        )
        .await
        {
            FetchResponse::Passthrough(response) => passthrough_response(response),
            FetchResponse::Cache(stream) => {
                let reader = {
                    let mut cache = self.cache.lock();
                    let length = stream.length;
                    let reader = CacheReader::new(self.backend.clone(), path.to_owned(), stream);
                    cache.get_or_insert(path, length, reader)
                };

                reader_response(method, request_range, &reader)
            }
            FetchResponse::Err => None,
        }
    }
}

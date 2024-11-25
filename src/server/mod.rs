use std::ops::Range;
use std::sync::Arc;

use axum::response::Response;
use http::{Method, StatusCode};
use parking_lot::Mutex;

use crate::request::PlainBackend;
use crate::response::{fetch, CacheReader, FetchResponse};
use crate::transient_cache::TransientCache;
use response::{error_response, passthrough_response, reader_response};

mod header;
mod response;

type ReaderType = CacheReader<PlainBackend>;
type ReaderArc = Arc<CacheReader<PlainBackend>>;

pub struct Server {
    // NOTE: this must be Arc because the cache item lifetimes
    // need to be independent of this server
    backend: Arc<PlainBackend>,
    cache: Mutex<TransientCache<ReaderArc>>,
    max_length_for_cached_objects: usize,
}

impl Server {
    pub fn new(transient_cache_size: usize, max_length_for_cached_objects: usize) -> Self {
        Self {
            backend: Arc::new(PlainBackend::create("https://example.com").unwrap()),
            cache: Mutex::new(TransientCache::new(transient_cache_size)),
            max_length_for_cached_objects,
        }
    }

    pub async fn stream_response(
        &self,
        method: &Method,
        path: &str,
        range: &Option<Range<usize>>,
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
        {
            let mut cache = self.cache.lock();

            if let Some(reader) = cache.get(path) {
                return reader_response(method, range, reader);
            }
        }

        // The item was not in the cache, so make a request.
        match fetch(
            &*self.backend,
            method,
            path,
            range,
            self.max_length_for_cached_objects,
        )
        .await
        {
            FetchResponse::Passthrough(response) => passthrough_response(response),
            FetchResponse::Cache(stream) => {
                let reader = {
                    let mut cache = self.cache.lock();
                    let length = stream.length;
                    let reader = ReaderType::new(self.backend.clone(), path.to_owned(), stream);
                    cache.get_or_insert(path, length, reader)
                };

                reader_response(method, range, reader)
            }
            FetchResponse::Err => None,
        }
    }
}

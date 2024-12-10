use parking_lot::Mutex;
use std::sync::Arc;

use crate::response_builder::ResponseBuilder;
use crate::types::*;
use sized_ttl_cache::{Entry, SizedTTLCache};

// Main service for cache streamer.
pub struct Service<K, R>
where
    K: Ord + 'static,
    R: Response,
{
    backend: Arc<dyn RequestBackend<K, R>>,
    cache: Mutex<SizedTTLCache<K, R::Timepoint, ResponseBuilder<R>>>,
}

impl<K, R> Service<K, R>
where
    K: Ord + 'static,
    R: Response,
{
    pub fn new(backend: Arc<dyn RequestBackend<K, R>>, cache_capacity: usize) -> Self {
        Self {
            backend,
            cache: Mutex::new(SizedTTLCache::with_capacity(cache_capacity)),
        }
    }

    /// Get a response with the given current time, request key, and request range.
    pub async fn call(&self, time: &R::Timepoint, key: K, range: &RequestRange) -> Result<R>
    where
        K: ToOwned<Owned = K>,
    {
        // Try to get the item from cache.
        //
        // The cache may also contain partial items which have not finished streaming yet.
        // This is fine, because our response will fetch unfinished bytes and continue
        // to feed the stream.
        if let Some(item) = self.cache.lock().get(time, &key) {
            return Ok(item.stream(range));
        }

        // The item was not in the cache, so make a request.
        let requester = self.backend.create_for_key(key.to_owned());

        // Even if the request is potentially cacheable, we only cache requests that return
        // some form of valid response range. Without this, we can't support suffix queries
        // correctly.
        let (response, range, expire_time, data) = match requester.fetch(range).await? {
            ResponseType::Cache(response, range, expire_time, data) => {
                (response, range, expire_time, data)
            }
            ResponseType::Passthrough(r) => return Ok(r),
        };

        // The response builder will return a stream here built from the current response,
        // avoiding the need to make a second request.
        let (stream, item) = ResponseBuilder::new(response, &range, data, requester);

        // Insert the new builder into the cache.
        let entry = Entry::from_parts(range.bytes_len, expire_time, item);
        self.cache.lock().get_or_insert(time, &key, entry);

        Ok(stream)
    }
}
